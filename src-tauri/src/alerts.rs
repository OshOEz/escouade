//! What a notification of an agent says: the body of its alert, on one line and cut short (a toast shows
//! little more), in the interface's language. Only text: `notify` tells it, with the window.

use crate::i18n::{self, Lang};
use crate::paths::relative_slash;
use serde_json::Value;

/// The most a notification says: a toast shows little more.
const MAX_BODY: usize = 120;

/// What a tool acts on, in the order the input's fields tell it best: a search shows its pattern
/// before its folder, a web fetch its URL before its prompt.
const TOOL_TARGETS: [&str; 9] = [
    "command",
    "file_path",
    "notebook_path",
    "url",
    "query",
    "pattern",
    "path",
    "description",
    "prompt",
];

/// `text` cut to `max` characters, ending with "…" when it was longer.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// `text` on one line: its runs of spaces and its line breaks become single spaces.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What a tool is asked to act on, on one line (a file shown from the agent's folder); empty when
/// its input has none of the fields that tell it.
fn tool_target(input: &Value, cwd: &str) -> String {
    TOOL_TARGETS
        .iter()
        .find_map(|key| {
            let value = one_line(input[*key].as_str()?);
            let is_path = matches!(*key, "file_path" | "notebook_path" | "path");
            let shown = if is_path {
                relative_slash(cwd, &value)
            } else {
                value
            };
            (!shown.is_empty()).then_some(shown)
        })
        .unwrap_or_default()
}

/// "Autoriser Bash : npm test ?": the tool, and what it is asked to act on (the command, the file
/// shown from the agent's folder, the URL, the search…), the whole in 120 characters. A plan to
/// approve is worded as the app words it: "Approuver le plan : <first line of the plan> ?".
pub fn permission_body(tool: &str, input: &Value, cwd: &str) -> String {
    permission_body_in(i18n::ui(), tool, input, cwd)
}

/// `permission_body` in `lang`: “Allow Bash: npm test?”, “Approve the plan: …?”.
fn permission_body_in(lang: Lang, tool: &str, input: &Value, cwd: &str) -> String {
    let (head, target) = if tool == "ExitPlanMode" {
        let plan = input["plan"].as_str().and_then(first_plain_line);
        (
            tr_in!(lang, "Approuver le plan", "Approve the plan"),
            plan.unwrap_or_default(),
        )
    } else if tool == "REPL" {
        // Code Mode's code (ESC-553), by what it says it does: its JavaScript is never a line to read here.
        (
            tr_in!(lang, "Lancer du code", "Run code"),
            input["description"]
                .as_str()
                .map(one_line)
                .unwrap_or_default(),
        )
    } else {
        (
            tr_in!(lang, "Autoriser {tool}", "Allow {tool}"),
            tool_target(input, cwd),
        )
    };
    let framed = |t: &str| tr_in!(lang, "{head} : {t} ?", "{head}: {t}?");
    if target.is_empty() {
        return clip(&tr_in!(lang, "{head} ?", "{head}?"), MAX_BODY);
    }
    // What frames the target (« : » before it, « ? » after it) is always shown.
    let room = MAX_BODY.saturating_sub(framed("").chars().count()).max(10);
    framed(&clip(&target, room))
}

/// The text of the question an agent asks (the first, when it asks several), in 120 characters;
/// without one, that it waits, named `speaker` as its harness names its agents (`Claude`).
pub fn question_body(speaker: &str, input: &Value) -> String {
    question_body_in(i18n::ui(), speaker, input)
}

fn question_body_in(lang: Lang, speaker: &str, input: &Value) -> String {
    let text = input["questions"][0]["question"]
        .as_str()
        .map(|q| {
            q.lines()
                .map(plain_line)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if text.is_empty() {
        tr_in!(
            lang,
            "{speaker} attend ta réponse",
            "{speaker} is waiting for your answer"
        )
    } else {
        clip(&text, MAX_BODY)
    }
}

/// The first line of the agent's final reply, without Markdown, in 120 characters; "Tâche
/// terminée" when it has no words.
pub fn done_body(reply: &str) -> String {
    done_body_in(i18n::ui(), reply)
}

fn done_body_in(lang: Lang, reply: &str) -> String {
    first_plain_line(reply).map_or_else(
        || tr_in!(lang, "Tâche terminée", "Task done"),
        |l| clip(&l, MAX_BODY),
    )
}

/// The first line of `text` that says something, as plain words: blank lines, rules and fenced
/// code blocks do not count.
fn first_plain_line(text: &str) -> Option<String> {
    let mut in_code = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let plain = plain_line(line);
        if !plain.is_empty() {
            return Some(plain);
        }
    }
    None
}

/// "Erreur : <reason>", the reason's first line, in 120 characters.
pub fn error_body(reason: &str) -> String {
    error_body_in(i18n::ui(), reason)
}

fn error_body_in(lang: Lang, reason: &str) -> String {
    let reason = reason
        .lines()
        .map(one_line)
        .find(|l| !l.is_empty())
        .unwrap_or_else(|| tr_in!(lang, "l'agent s'est arrêté", "the agent stopped"));
    clip(
        &tr_in!(lang, "Erreur : {reason}", "Error: {reason}"),
        MAX_BODY,
    )
}

/// One line of Markdown as plain words: without its heading, quote, list or checkbox marker, its
/// emphasis and code marks, its links shown by their text. Empty for a rule or a table's border.
fn plain_line(line: &str) -> String {
    let mut rest = line.trim();
    let border = |c: char| matches!(c, '-' | '*' | '_' | '=' | '|' | ':' | ' ');
    if rest.chars().filter(|c| *c != ' ').count() >= 3 && rest.chars().all(border) {
        return String::new();
    }
    loop {
        let before = rest;
        rest = rest.trim_start();
        if let Some(quoted) = rest.strip_prefix('>') {
            rest = quoted;
        }
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && rest[hashes..].starts_with(' ') {
            rest = &rest[hashes..];
        }
        for bullet in ["- ", "* ", "+ "] {
            if let Some(item) = rest.strip_prefix(bullet) {
                rest = item;
            }
        }
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && (rest[digits..].starts_with(". ") || rest[digits..].starts_with(") ")) {
            rest = &rest[digits + 2..];
        }
        for task in ["[ ] ", "[x] ", "[X] "] {
            if let Some(item) = rest.strip_prefix(task) {
                rest = item;
            }
        }
        if rest == before {
            break;
        }
    }
    one_line(&strip_inline(rest))
}

/// Where, from `from`, a run of exactly `len` backticks begins: the end of a code span.
fn closing_fence(chars: &[char], from: usize, len: usize) -> Option<usize> {
    let mut at = from;
    while at < chars.len() {
        if chars[at] != '`' {
            at += 1;
            continue;
        }
        let run = chars[at..].iter().take_while(|x| **x == '`').count();
        if run == len {
            return Some(at);
        }
        at += run;
    }
    None
}

/// Inline Markdown as plain words: `[text](url)` and `![alt](url)` read as their text, the marks of
/// bold, italic and strikethrough dropped, the backticks of code spans too (their text is kept as
/// it is). A star between spaces and an underscore inside a word (`snake_case`) are not marks.
fn strip_inline(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '[' | '!' => {
                let open = if c == '!' { i + 1 } else { i };
                let link = (chars.get(open) == Some(&'['))
                    .then(|| {
                        let close = chars[open..].iter().position(|x| *x == ']')? + open;
                        (chars.get(close + 1) == Some(&'(')).then_some(())?;
                        let end = chars[close..].iter().position(|x| *x == ')')? + close;
                        Some((open + 1..close, end))
                    })
                    .flatten();
                match link {
                    Some((label, end)) => {
                        out.push_str(&strip_inline(&chars[label].iter().collect::<String>()));
                        i = end + 1;
                    }
                    None => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '`' => {
                // A code span is copied as it is, only its backticks go: what looks like emphasis
                // or a link in it (`*.log`, `__init__`) is code. A span ends at a run of as many
                // backticks as opened it; one that never ends loses its backticks alone.
                let fence = chars[i..].iter().take_while(|x| **x == '`').count();
                let body = i + fence;
                match closing_fence(&chars, body, fence) {
                    Some(close) => {
                        out.extend(&chars[body..close]);
                        i = close + fence;
                    }
                    None => i = body,
                }
            }
            '*' | '_' | '~' => {
                let run = chars[i..].iter().take_while(|x| **x == c).count();
                let prev = i.checked_sub(1).map(|p| chars[p]);
                let next = chars.get(i + run).copied();
                let spaced = |x: Option<char>| x.is_none_or(char::is_whitespace);
                let word = |x: Option<char>| x.is_some_and(char::is_alphanumeric);
                let keep = run == 1
                    && match c {
                        '*' => spaced(prev) && spaced(next),
                        '_' => word(prev) && word(next),
                        _ => true,
                    };
                if keep {
                    out.push(c);
                }
                i += run;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_permission_names_the_tool_and_what_it_acts_on() {
        // A command, a file of the agent's folder (shown from it), a URL, a search.
        assert_eq!(
            permission_body("Bash", &json!({ "command": "rm -rf build" }), "C:/p"),
            "Autoriser Bash : rm -rf build ?"
        );
        assert_eq!(
            permission_body("Edit", &json!({ "file_path": "C:/p/src/app.ts" }), "C:/p"),
            "Autoriser Edit : src/app.ts ?"
        );
        assert_eq!(
            permission_body(
                "WebFetch",
                &json!({ "url": "https://example.com/doc", "prompt": "Résume" }),
                "C:/p"
            ),
            "Autoriser WebFetch : https://example.com/doc ?"
        );
        assert_eq!(
            permission_body(
                "WebSearch",
                &json!({ "query": "tauri notifications" }),
                "C:/p"
            ),
            "Autoriser WebSearch : tauri notifications ?"
        );
        // A file outside the folder keeps its path.
        assert_eq!(
            permission_body("Read", &json!({ "file_path": "D:/autre/notes.md" }), "C:/p"),
            "Autoriser Read : D:/autre/notes.md ?"
        );
    }

    /// ESC-553: Code Mode's code (`REPL`) asked by an `ask` rule, by what it says it does; the calls the code makes are
    /// asked under their own names, as they reach the hooks (`PermissionRequest` of `Bash`, 2.1.329).
    #[test]
    fn code_modes_code_is_asked_by_its_description_and_its_calls_by_their_names() {
        let code = json!({
            "code": "const r = await $`mkdir e-made`.nothrow();\nr.exitCode",
            "description": "Create e-made folder in working directory",
        });
        assert_eq!(
            permission_body_in(Lang::Fr, "REPL", &code, "C:/p"),
            "Lancer du code : Create e-made folder in working directory ?"
        );
        assert_eq!(
            permission_body_in(Lang::En, "REPL", &json!({ "code": "1" }), "C:/p"),
            "Run code?"
        );
        assert_eq!(
            permission_body_in(
                Lang::Fr,
                "Bash",
                &json!({ "command": "cd 'C:/p' && mkdir e-made" }),
                "C:/p"
            ),
            "Autoriser Bash : cd 'C:/p' && mkdir e-made ?"
        );
    }

    #[test]
    fn a_permission_command_is_read_on_one_line() {
        assert_eq!(
            permission_body(
                "Bash",
                &json!({ "command": "cd api &&\n  npm test\n" }),
                "C:/p"
            ),
            "Autoriser Bash : cd api && npm test ?"
        );
    }

    #[test]
    fn a_long_permission_is_cut_to_120_characters_and_still_asks() {
        let body = permission_body("Bash", &json!({ "command": "é".repeat(400) }), "C:/p");
        assert_eq!(body.chars().count(), 120);
        assert!(body.starts_with("Autoriser Bash : ééé"), "{body}");
        assert!(body.ends_with("… ?"), "{body}");
    }

    #[test]
    fn a_permission_without_anything_to_show_only_names_the_tool() {
        assert_eq!(
            permission_body("TodoWrite", &json!({ "todos": [] }), "C:/p"),
            "Autoriser TodoWrite ?"
        );
        assert_eq!(
            permission_body("Bash", &json!({ "command": "  " }), "C:/p"),
            "Autoriser Bash ?"
        );
    }

    #[test]
    fn a_search_permission_shows_the_pattern_not_the_folder() {
        assert_eq!(
            permission_body(
                "Grep",
                &json!({ "pattern": "TODO", "path": "C:/p/src" }),
                "C:/p"
            ),
            "Autoriser Grep : TODO ?"
        );
    }

    #[test]
    fn a_plan_asks_for_its_approval_in_the_apps_words() {
        // The first line of the plan, without Markdown.
        assert_eq!(
            permission_body(
                "ExitPlanMode",
                &json!({ "plan": "\n## Ajouter le **filtre** PDF\n\n1. Écrire le test\n2. Le coder" }),
                "C:/p"
            ),
            "Approuver le plan : Ajouter le filtre PDF ?"
        );
        // No plan to read: the question alone.
        assert_eq!(
            permission_body("ExitPlanMode", &json!({}), "C:/p"),
            "Approuver le plan ?"
        );
        assert_eq!(
            permission_body("ExitPlanMode", &json!({ "plan": " \n---\n" }), "C:/p"),
            "Approuver le plan ?"
        );
        // Cut like any other, the question mark kept.
        let long = permission_body(
            "ExitPlanMode",
            &json!({ "plan": "Refaire ".repeat(40) }),
            "C:/p",
        );
        assert_eq!(long.chars().count(), 120);
        assert!(
            long.starts_with("Approuver le plan : Refaire Refaire") && long.ends_with("… ?"),
            "{long}"
        );
    }

    #[test]
    fn a_question_is_its_own_text() {
        let input = json!({ "questions": [
            { "question": "Quelle base de données ?", "header": "Base", "options": [] },
            { "question": "Et le cache ?", "options": [] },
        ] });
        assert_eq!(question_body("Claude", &input), "Quelle base de données ?");
        // On several lines, or dressed in Markdown: one plain line.
        assert_eq!(
            question_body(
                "Claude",
                &json!({ "questions": [{ "question": "Garder **`auth.ts`** ?\nOu le refaire ?" }] })
            ),
            "Garder auth.ts ? Ou le refaire ?"
        );
    }

    #[test]
    fn a_long_question_is_cut_to_120_characters() {
        let long = format!("Que faire de {} ?", "tous ces fichiers ".repeat(20));
        let body = question_body("Claude", &json!({ "questions": [{ "question": long }] }));
        assert_eq!(body.chars().count(), 120);
        assert!(body.starts_with("Que faire de tous ces fichiers"), "{body}");
        assert!(body.ends_with('…'), "{body}");
    }

    #[test]
    fn a_question_without_text_keeps_the_old_words() {
        assert_eq!(
            question_body("Claude", &json!({ "questions": [] })),
            "Claude attend ta réponse"
        );
        assert_eq!(
            question_body("Claude", &json!({})),
            "Claude attend ta réponse"
        );
    }

    #[test]
    fn the_end_of_a_turn_tells_the_first_line_of_the_reply() {
        assert_eq!(
            done_body("J’ai corrigé le bug du filtre.\n\nLe test passe."),
            "J’ai corrigé le bug du filtre."
        );
        // Blank lines first do not count.
        assert_eq!(done_body("\n\n  Terminé.  \nSuite"), "Terminé.");
    }

    #[test]
    fn the_first_line_of_a_reply_is_read_without_markdown() {
        let plain = |md: &str| done_body(md);
        assert_eq!(
            plain("## Résumé des changements\n\nDétail"),
            "Résumé des changements"
        );
        assert_eq!(
            plain("**Fait** : le filtre accepte les _PDF_."),
            "Fait : le filtre accepte les PDF."
        );
        assert_eq!(
            plain("- Ajouté `parse()` dans [utils](src/utils.ts)"),
            "Ajouté parse() dans utils"
        );
        assert_eq!(plain("3. ~~Ancien~~ nouveau test"), "Ancien nouveau test");
        assert_eq!(plain("> Une citation\nsuite"), "Une citation");
        assert_eq!(plain("- [x] Tests écrits"), "Tests écrits");
        assert_eq!(
            plain("![schéma](a.png) voir ci-dessous"),
            "schéma voir ci-dessous"
        );
        // Words with an underscore or a lone star are left alone.
        assert_eq!(
            plain("Renommé snake_case_name et 2 * 3"),
            "Renommé snake_case_name et 2 * 3"
        );
    }

    #[test]
    fn what_is_between_backticks_is_copied_as_it_is() {
        // Stars and underscores in code are no emphasis; only the backticks go.
        assert_eq!(
            done_body("Ignore `*.log` et `src/**/*.ts` dans `__init__.py`."),
            "Ignore *.log et src/**/*.ts dans __init__.py."
        );
        assert_eq!(
            done_body("Appelle `a_b_c(*args)` !"),
            "Appelle a_b_c(*args) !"
        );
        // Not read as a link either; a longer fence holds a single backtick.
        assert_eq!(done_body("Voir `[a](b)`"), "Voir [a](b)");
        assert_eq!(done_body("Écris ``a ` b`` ici"), "Écris a ` b ici");
        // A backtick that never closes is just dropped, and the text after it is read as usual.
        assert_eq!(
            done_body("Reste `ouvert et **gras**"),
            "Reste ouvert et gras"
        );
        assert_eq!(
            question_body(
                "Claude",
                &json!({ "questions": [{ "question": "Garder `*.tmp` ?" }] })
            ),
            "Garder *.tmp ?"
        );
    }

    #[test]
    fn a_reply_that_opens_with_a_rule_or_a_code_block_is_read_from_its_first_words() {
        assert_eq!(done_body("---\nBilan : tout passe."), "Bilan : tout passe.");
        assert_eq!(
            done_body("```rust\nfn main() {}\n```\nLe code ci-dessus compile."),
            "Le code ci-dessus compile."
        );
    }

    #[test]
    fn a_long_first_line_is_cut_to_120_characters() {
        let body = done_body(&format!("Voici {}", "un très long résumé ".repeat(20)));
        // A space where it was cut does not stay before the ellipsis.
        assert!((118..=120).contains(&body.chars().count()), "{body}");
        assert!(body.ends_with('…') && !body.contains(" …"), "{body}");
    }

    #[test]
    fn a_turn_without_words_is_just_done() {
        assert_eq!(done_body(""), "Tâche terminée");
        assert_eq!(done_body("  \n \n"), "Tâche terminée");
        assert_eq!(done_body("---\n```\ncode\n```"), "Tâche terminée");
    }

    #[test]
    fn an_error_tells_its_reason_on_one_line() {
        assert_eq!(
            error_body("Claude Code s'est arrêté (code 3)"),
            "Erreur : Claude Code s'est arrêté (code 3)"
        );
        assert_eq!(
            error_body("\n API Error: 500\ntrace…"),
            "Erreur : API Error: 500"
        );
        assert_eq!(error_body("  "), "Erreur : l'agent s'est arrêté");
        let long = error_body(&"x".repeat(300));
        assert_eq!(long.chars().count(), 120);
        assert!(
            long.starts_with("Erreur : xxx") && long.ends_with('…'),
            "{long}"
        );
    }

    #[test]
    fn a_notification_says_it_in_english_with_the_punctuation_of_english() {
        use crate::i18n::Lang::En;
        assert_eq!(
            permission_body_in(En, "Bash", &json!({ "command": "npm test" }), "C:/p"),
            "Allow Bash: npm test?"
        );
        assert_eq!(
            permission_body_in(En, "TodoWrite", &json!({ "todos": [] }), "C:/p"),
            "Allow TodoWrite?"
        );
        assert_eq!(
            permission_body_in(
                En,
                "ExitPlanMode",
                &json!({ "plan": "# Fix the filter" }),
                "C:/p"
            ),
            "Approve the plan: Fix the filter?"
        );
        // The room left for the target still ends the body with « ? », 120 characters in all.
        let long = permission_body_in(En, "Bash", &json!({ "command": "x".repeat(400) }), "C:/p");
        assert_eq!(long.chars().count(), 120);
        assert!(
            long.starts_with("Allow Bash: xxx") && long.ends_with("…?"),
            "{long}"
        );
        assert_eq!(
            question_body_in(En, "Claude", &json!({})),
            "Claude is waiting for your answer"
        );
        // Another harness: its agents' name.
        assert_eq!(
            question_body_in(En, "Codex", &json!({})),
            "Codex is waiting for your answer"
        );
        assert_eq!(
            question_body_in(Lang::Fr, "Codex", &json!({})),
            "Codex attend ta réponse"
        );
        assert_eq!(done_body_in(En, "---"), "Task done");
        assert_eq!(error_body_in(En, "API Error: 500"), "Error: API Error: 500");
        assert_eq!(error_body_in(En, " "), "Error: the agent stopped");
    }
}
