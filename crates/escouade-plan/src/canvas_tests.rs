//! Tests of the canvas summary (`canvas.rs`): the skills and plugins of an agent and of its subagents.

use crate::canvas::*;
use crate::plan::Change;
use serde_json::json;

#[test]
fn a_skill_is_named_by_its_input_and_a_plugin_by_its_server() {
    assert_eq!(
        kit_of("Skill", &json!({"skill": " superpowers:writing-plans "})),
        Some((KitKind::Skill, "superpowers:writing-plans".into()))
    );
    assert_eq!(
        kit_of("mcp__github__get_pr", &json!({})),
        Some((KitKind::Plugin, "github".into()))
    );
    assert_eq!(kit_of("Read", &json!({})), None);
    assert_eq!(kit_of("Skill", &json!({})), None);
    assert_eq!(kit_of("mcp____x", &json!({})), None);
}

#[test]
fn counts_the_calls_and_says_which_one_runs() {
    let mut c = Canvas::default();
    assert_eq!(
        c.on_tool_use(None, "k1", "mcp__github__a", &json!({}), 1),
        Change::Saved
    );
    assert_eq!(
        c.on_tool_use(None, "k2", "mcp__github__b", &json!({}), 2),
        Change::View
    );
    assert_eq!((c.tools[0].calls, c.tools[0].running), (2, 2));
    assert_eq!(c.on_tool_result("k1", false, "", 3), Change::View);
    assert_eq!(c.tools[0].running, 1);
    assert_eq!(c.on_tool_result("nope", false, "", 3), Change::None);
}

#[test]
fn a_call_seen_twice_counts_once() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1);
    assert_eq!(
        c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1),
        Change::None
    );
    assert_eq!(c.tools[0].calls, 1);
}

#[test]
fn keeps_the_tools_of_each_subagent_apart() {
    let mut c = Canvas::default();
    c.on_tool_use(Some("a1"), "k1", "Skill", &json!({"skill": "x"}), 1);
    assert!(c.tools.is_empty());
    assert_eq!(c.subs[0].call, "a1");
    assert_eq!(c.subs[0].tools[0].name, "x");
    assert_eq!(c.on_tool_result("k1", false, "", 2), Change::View);
    assert_eq!(c.subs[0].tools[0].running, 0);
}

#[test]
fn nothing_runs_after_a_reload_or_the_end_of_the_process() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1);
    let back: Canvas = serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
    assert_eq!((back.tools[0].calls, back.tools[0].running), (1, 0));
    assert_eq!(c.close_running(9), Change::View);
    assert_eq!(c.tools[0].running, 0);
    assert_eq!(c.close_running(9), Change::None);
    assert_eq!(c.on_tool_result("k1", false, "", 10), Change::None);
}

#[test]
fn keeps_at_most_40_tools_a_thread() {
    let mut c = Canvas::default();
    for i in 0..45 {
        c.on_tool_use(
            None,
            &format!("k{i}"),
            &format!("mcp__s{i}__t"),
            &json!({}),
            i,
        );
    }
    assert_eq!(c.tools.len(), MAX_KIT);
    assert!(c.tools.iter().all(|t| t.name != "s0"));
}

#[test]
fn keeps_at_most_30_subagent_threads() {
    let mut c = Canvas::default();
    for i in 0..32 {
        c.on_tool_use(
            Some(&format!("a{i}")),
            &format!("k{i}"),
            "Skill",
            &json!({"skill": "x"}),
            i,
        );
    }
    assert_eq!(c.subs.len(), MAX_THREADS);
    assert_eq!(c.subs[0].call, "a2");
    assert!(!c.is_empty());
}

#[test]
fn a_server_named_with_two_underscores_is_kept_whole() {
    assert_eq!(
        kit_of("mcp__my__srv__get", &json!({})),
        Some((KitKind::Plugin, "my__srv".into()))
    );
    assert_eq!(
        kit_of("mcp__plugin_superpowers_chrome__click", &json!({})),
        Some((KitKind::Plugin, "plugin_superpowers_chrome".into()))
    );
}

#[test]
fn keeps_the_time_of_the_first_call() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 3);
    c.on_tool_use(None, "k2", "Skill", &json!({"skill": "x"}), 8);
    assert_eq!((c.tools[0].first_at, c.tools[0].last_at), (3, 8));
}

#[test]
fn the_caps_spare_what_runs() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "run", "mcp__s0__t", &json!({}), 0);
    for i in 1..MAX_KIT as i64 {
        let id = format!("k{i}");
        c.on_tool_use(None, &id, &format!("mcp__s{i}__t"), &json!({}), i);
        c.on_tool_result(&id, false, "", i);
    }
    c.on_tool_use(None, "new", "mcp__new__t", &json!({}), 99);
    assert!(c.tools.iter().any(|t| t.name == "s0"));
    assert!(c.tools.iter().all(|t| t.name != "s1"));

    let mut c = Canvas::default();
    c.on_tool_use(Some("a0"), "run", "Skill", &json!({"skill": "x"}), 0);
    for i in 1..=MAX_THREADS {
        let id = format!("k{i}");
        c.on_tool_use(
            Some(&format!("a{i}")),
            &id,
            "Skill",
            &json!({"skill": "x"}),
            i as i64,
        );
        c.on_tool_result(&id, false, "", i as i64);
    }
    assert_eq!(c.subs.len(), MAX_THREADS);
    assert_eq!(
        (c.subs[0].call.as_str(), c.subs[1].call.as_str()),
        ("a0", "a2")
    );
}
