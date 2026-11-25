//! What the « Workspace » shows of an agent without loading its conversation (spec
//! docs/superpowers/specs/2026-11-25-vue-workspace-design.md, « D'où viennent les données ») : the
//! skills and plugins it used, its own and each of its subagents', with their calls and the ones
//! running. Fed by `agent.rs` at each tool call and result, as the plan; it goes out with the agent
//! (`AgentMeta::canvas`). Never reset: it says what the agent used in its life, its caps bound it.

use crate::plan::{Change, Seen};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The most skills and plugins a thread (the agent, a subagent) keeps: the least recent go.
pub const MAX_KIT: usize = 40;
/// The most subagent threads kept: the first come go.
pub const MAX_THREADS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KitKind {
    /// A call of the tool `Skill`.
    #[default]
    Skill,
    /// A server of tools: the `<server>` of `mcp__<server>__<tool>`.
    Plugin,
}

/// A skill or a plugin a thread used.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitUse {
    pub kind: KitKind,
    pub name: String,
    pub calls: u32,
    /// Its calls running now: none survives a reload.
    #[serde(skip_deserializing)]
    pub running: u32,
    pub last_at: i64,
}

/// The skills and plugins of a subagent, by its call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThreadKit {
    pub call: String,
    pub tools: Vec<KitUse>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Canvas {
    /// The agent's own.
    pub tools: Vec<KitUse>,
    /// Each subagent's, in the order they came.
    pub subs: Vec<ThreadKit>,
    /// The latest calls looked at: a frame that comes twice counts once.
    #[serde(skip)]
    seen: Seen,
    /// The calls running, by id: their thread (`None`: the agent's) and their tool.
    #[serde(skip)]
    open: HashMap<String, (Option<String>, KitKind, String)>,
}

/// The skill or plugin a call of `name` uses, if any.
pub fn kit_of(name: &str, input: &Value) -> Option<(KitKind, String)> {
    if name == "Skill" {
        let skill = input["skill"].as_str()?.trim();
        return (!skill.is_empty()).then(|| (KitKind::Skill, skill.to_string()));
    }
    let (server, _) = name.strip_prefix("mcp__")?.split_once("__")?;
    (!server.is_empty()).then(|| (KitKind::Plugin, server.to_string()))
}

impl Canvas {
    /// The tools of a thread, `None` the agent's.
    fn thread(&mut self, call: Option<&str>) -> Option<&mut Vec<KitUse>> {
        match call {
            None => Some(&mut self.tools),
            Some(call) => self
                .subs
                .iter_mut()
                .find(|t| t.call == call)
                .map(|t| &mut t.tools),
        }
    }

    /// A tool call, `parent` being the call of the subagent it comes from.
    pub fn on_tool_use(
        &mut self,
        parent: Option<&str>,
        id: &str,
        name: &str,
        input: &Value,
        now: i64,
    ) -> Change {
        let Some((kind, kit)) = kit_of(name, input) else {
            return Change::None;
        };
        if !self.seen.first_time(id) {
            return Change::None;
        }
        if let Some(call) = parent {
            if !self.subs.iter().any(|t| t.call == call) {
                if self.subs.len() >= MAX_THREADS {
                    self.subs.remove(0);
                }
                self.subs.push(ThreadKit {
                    call: call.to_string(),
                    tools: Vec::new(),
                });
            }
        }
        self.open.insert(
            id.to_string(),
            (parent.map(str::to_string), kind, kit.clone()),
        );
        let tools = self.thread(parent).expect("made above");
        if let Some(t) = tools.iter_mut().find(|t| t.kind == kind && t.name == kit) {
            t.calls += 1;
            t.running += 1;
            t.last_at = now;
            return Change::View;
        }
        if tools.len() >= MAX_KIT {
            let oldest = (0..tools.len()).min_by_key(|&i| tools[i].last_at);
            tools.remove(oldest.expect("full"));
        }
        tools.push(KitUse {
            kind,
            name: kit,
            calls: 1,
            running: 1,
            last_at: now,
        });
        Change::Saved
    }

    /// A tool's result. `is_error` and `text` are for what comes next (artifacts, commands).
    pub fn on_tool_result(&mut self, id: &str, _is_error: bool, _text: &str, now: i64) -> Change {
        let Some((thread, kind, name)) = self.open.remove(id) else {
            return Change::None;
        };
        let Some(t) = self
            .thread(thread.as_deref())
            .and_then(|tools| tools.iter_mut().find(|t| t.kind == kind && t.name == name))
        else {
            return Change::None;
        };
        t.running = t.running.saturating_sub(1);
        t.last_at = now;
        Change::View
    }

    /// The process is gone: nothing it called runs any more.
    pub fn close_running(&mut self, _now: i64) -> Change {
        let mut change = Change::None;
        let subs = self.subs.iter_mut().flat_map(|t| t.tools.iter_mut());
        for t in self.tools.iter_mut().chain(subs) {
            if t.running > 0 {
                t.running = 0;
                change = Change::View;
            }
        }
        self.open.clear();
        change
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.subs.iter().all(|t| t.tools.is_empty())
    }
}
