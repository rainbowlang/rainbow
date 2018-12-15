use crate::arena::ArenaId;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    /// Push a constant value onto the values stack
    PushPrimitive {
        id: ArenaId,
    },
    PushVar {
        id: ArenaId,
    },
    PushProp {
        id: ArenaId,
    },
    PushKeyword {
        id: ArenaId,
    },
    MkList {
        size: u16,
    },
    MkRecord {
        size: u16,
    },
    MkBlock {
        argc: u8,
        skip: u16,
    },
    Bind {
        id: ArenaId,
    },
    CallFunction {
        argc: u16,
    },
}
