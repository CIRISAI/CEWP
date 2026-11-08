//! Core state processor types for CC §4.5.13.

use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub tier: String,
    pub room: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub family: String,
    pub verb: String,
    pub signer: String,
    pub signer_type: Option<String>,
    pub object: String,
    pub scope: Option<Scope>,
    pub subject: Option<String>,
    pub stance: Option<String>,
    pub ranking: Option<Vec<String>>,
    pub asserted_at: u64,
    pub note: Option<String>,
    pub role: Option<String>,
    pub job: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldRow {
    pub row: Row,
    pub first_held: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fraction {
    pub num: u64,
    pub den: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub verb: String,
    pub from: String,
    pub to: String,
    pub roles: Vec<String>,
    pub policy: String,
    pub policy_rows: Vec<String>,
    pub widens: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub family: String,
    pub kind: String,
    pub enact: Option<Fraction>,
    pub cancel: Option<Fraction>,
    pub floor: Option<Fraction>,
    pub irreversible: bool,
    pub reverse_path: bool,
    pub cancel_over: Option<String>,
    pub base_law: Option<String>,
    pub base: Option<String>,
    pub at_window: Option<String>,
    pub window_secs: Option<u64>,
    pub window_class: Option<String>,
    pub corridor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub row: Row,
    pub transition: Transition,
    pub first_held: u64,
    pub policy: Option<Policy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Refusal {
    AlreadyHeld,
    FamilyMismatch,
    NoTransition,
    Closed,
    NoStanding,
    Unregistered,
    NoSuchRoom,
    SubjectOutsideAudience,
    Halted,
    UnknownFamily,
    NotEnabled,
    SelfLoop,
    PolicyRefused,
    ScoreOutsideDeclaredPolarity,
    Other(String),
}

impl Refusal {
    pub fn as_str(&self) -> &str {
        match self {
            Refusal::AlreadyHeld => "already_held",
            Refusal::FamilyMismatch => "family_mismatch",
            Refusal::NoTransition => "no_transition",
            Refusal::Closed => "closed",
            Refusal::NoStanding => "no_standing",
            Refusal::Unregistered => "unregistered",
            Refusal::NoSuchRoom => "no_such_room",
            Refusal::SubjectOutsideAudience => "subject_outside_audience",
            Refusal::Halted => "halted",
            Refusal::UnknownFamily => "unknown_family",
            Refusal::NotEnabled => "not_enabled",
            Refusal::SelfLoop => "self_loop",
            Refusal::PolicyRefused => "policy_refused",
            Refusal::ScoreOutsideDeclaredPolarity => "score_outside_declared_polarity",
            Refusal::Other(s) => s.as_str(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl PartialEq<str> for Refusal {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Refusal {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl From<&str> for Refusal {
    fn from(s: &str) -> Self {
        match s {
            "already_held" => Refusal::AlreadyHeld,
            "family_mismatch" => Refusal::FamilyMismatch,
            "no_transition" => Refusal::NoTransition,
            "closed" => Refusal::Closed,
            "no_standing" => Refusal::NoStanding,
            "unregistered" => Refusal::Unregistered,
            "no_such_room" => Refusal::NoSuchRoom,
            "subject_outside_audience" => Refusal::SubjectOutsideAudience,
            "halted" => Refusal::Halted,
            "unknown_family" => Refusal::UnknownFamily,
            "not_enabled" => Refusal::NotEnabled,
            "self_loop" => Refusal::SelfLoop,
            "policy_refused" => Refusal::PolicyRefused,
            "score_outside_declared_polarity" => Refusal::ScoreOutsideDeclaredPolarity,
            other => Refusal::Other(other.to_string()),
        }
    }
}

impl From<String> for Refusal {
    fn from(s: String) -> Self {
        Refusal::from(s.as_str())
    }
}

pub type StepResult = Result<Plan, Refusal>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub family: Option<String>,
    pub binds: HashMap<String, String>,
    pub refusal: Option<String>,
    pub ok: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TargetRef<'a> {
    pub id: Option<&'a str>,
    pub object: Option<&'a str>,
    pub subject: Option<&'a str>,
    pub author: Option<&'a str>,
    pub room: Option<&'a str>,
    pub signer_type: Option<&'a str>,
    pub tier: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voices {
    pub for_voices: Vec<String>,
    pub against_voices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    pub voices: Option<i64>,
    pub until: Option<u64>,
    pub ballots: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Enacted,
    EnactedIrreversible,
    Cancelled,
    Reversed,
    Stands,
    Acting,
    Seated(String),
}

impl Outcome {
    pub fn as_str(&self) -> &str {
        match self {
            Outcome::Enacted => "enacted",
            Outcome::EnactedIrreversible => "enacted_irreversible",
            Outcome::Cancelled => "cancelled",
            Outcome::Reversed => "reversed",
            Outcome::Stands => "stands",
            Outcome::Acting => "acting",
            Outcome::Seated(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub policy: Policy,
    pub transition: Transition,
    pub since: u64,
    pub opener: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ballot {
    pub signer: String,
    pub ranking: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectState {
    pub id: String,
    pub family: String,
    pub raw_family: Option<String>,
    pub binds: HashMap<String, String>,
    pub state: String,
    pub rows: Vec<HeldRow>,
    pub scope: Scope,
    pub author: String,
    pub subject: Option<String>,
    pub pending: Option<Pending>,
    pub outcome: Option<Outcome>,
    pub missing: Option<Missing>,
    pub voices: Option<Voices>,
    pub last: Option<HeldRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomState {
    pub id: String,
    pub founders: HashSet<String>,
    pub members: HashSet<String>,
    pub removed: HashSet<String>,
    pub moderators: HashSet<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaltState {
    pub latched: bool,
    pub by: Option<String>,
    pub since: Option<u64>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldState {
    pub objects: HashMap<String, ObjectState>,
    pub object_order: Vec<String>,
    pub registered: HashSet<String>,
    pub rooms: HashMap<String, RoomState>,
    pub room_order: Vec<String>,
    pub holders: HashSet<String>,
    pub nodes_of: HashMap<String, HashSet<String>>,
    pub halt: HaltState,
    pub roles_of: HashMap<String, HashSet<String>>,
    pub system_identities: HashSet<String>,
    pub owners_of: HashMap<String, HashSet<String>>,
    pub attesters: HashSet<String>,
    pub now: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub kind: String,
    pub object: String,
    pub family: String,
    pub verb: String,
    pub to: String,
    pub policy: String,
    pub confirm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueStart {
    pub kind: String,
    pub family: String,
    pub verb: String,
    pub to: String,
    pub policy: String,
    pub confirm: bool,
    pub found: Option<bool>,
    pub room: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    pub items: Vec<QueueItem>,
    pub starts: Vec<QueueStart>,
    pub fold: FoldState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedObject {
    pub object: String,
    pub family: String,
    pub from: String,
    pub to: String,
    pub outcome: Option<Outcome>,
    pub pending: Option<Missing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDiff {
    pub room: String,
    pub joined: Vec<String>,
    pub left: Vec<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewVerdict {
    pub row: String,
    pub ok: bool,
    pub plan: Option<Plan>,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewImpact {
    pub direction: String,
    pub audience: Vec<String>,
    pub undo: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewResult {
    pub verdicts: Vec<PreviewVerdict>,
    pub changed: Vec<ChangedObject>,
    pub rooms: Vec<RoomDiff>,
    pub impact: PreviewImpact,
}
