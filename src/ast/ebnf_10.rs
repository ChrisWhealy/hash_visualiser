use crate::{ast::ebnf_11::Expr, lexer::duration_unit::DurationUnit};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// §10 Effects
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone)]
pub enum Effect {
    Set(SetEffect),
    Animate(AnimateEffect),
    Emit(EmitEffect),
    Reroute(RerouteEffect),
    Let(LetBinding),
}

#[derive(Debug, Clone)]
pub enum SetEffect {
    Prop { name: String, value: Expr },
    Var { name: String, value: Expr },
    Bare(String),
}

#[derive(Debug, Clone)]
pub struct AnimateEffect {
    pub spec: AnimateSpec,
}

#[derive(Debug, Clone)]
pub enum AnimateSpec {
    FillPulse {
        colour: String,
        duration: Duration,
    },
    Transition {
        prop: String,
        from: Expr,
        to: Expr,
        duration: Duration,
    },
}

#[derive(Debug, Clone)]
pub struct Duration {
    pub value: u64,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone)]
pub struct EmitEffect {
    pub event: String,
    pub args: Vec<Expr>,
    pub target: Option<EmitTarget>,
}

#[derive(Debug, Clone)]
pub enum EmitTarget {
    All,
    Node(String),
    Via(String),
}

#[derive(Debug, Clone)]
pub struct RerouteEffect {
    pub wire: String,
    pub direction: RerouteDir,
    pub node: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RerouteDir {
    To,
    From,
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub value: Expr,
}
