#[derive(Clone, Display, Debug, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    Constant = 0,
    Nil,
    IConst0,
    IConst1,
    IConst2,
    IConst3,
    IConst4,
    IAdd,
    ISub,
    IMul,
    IDiv,
    FAdd,
    FSub,
    FMul,
    FDiv,
    IMod,
    FMod,
    I2F,
    F2I,
    Invert,
    StrConcat,
    T,
    F,
    And,
    Or,
    Negate,
    Coalesce,
    LT,
    LTE,
    GT,
    GTE,
    Eq,
    Neq,
    OptMk,
    MapMk,
    MapLoad,
    ArrMk,
    ArrLoad,
    ArrSlc,
    GStore,
    LStore0,
    LStore1,
    LStore2,
    LStore3,
    LStore4,
    LStore,
    UStore0,
    UStore1,
    UStore2,
    UStore3,
    UStore4,
    UStore,
    GLoad,
    LLoad0,
    LLoad1,
    LLoad2,
    LLoad3,
    LLoad4,
    LLoad,
    ULoad0,
    ULoad1,
    ULoad2,
    ULoad3,
    ULoad4,
    ULoad,
    Jump,
    JumpIfF,
    JumpB,
    Invoke,
    ClosureMk,
    CloseUpvalue,
    CloseUpvalueAndPop,
    Pop,
    PopN,
    Return,
}

impl From<&u8> for Opcode {
    fn from(i: &u8) -> Self {
        match i {
            0 => Opcode::Constant,
            1 => Opcode::Nil,
            2 => Opcode::IConst0,
            3 => Opcode::IConst1,
            4 => Opcode::IConst2,
            5 => Opcode::IConst3,
            6 => Opcode::IConst4,
            7 => Opcode::IAdd,
            8 => Opcode::ISub,
            9 => Opcode::IMul,
            10 => Opcode::IDiv,
            11 => Opcode::FAdd,
            12 => Opcode::FSub,
            13 => Opcode::FMul,
            14 => Opcode::FDiv,
            15 => Opcode::IMod,
            16 => Opcode::FMod,
            17 => Opcode::I2F,
            18 => Opcode::F2I,
            19 => Opcode::Invert,
            20 => Opcode::StrConcat,
            21 => Opcode::T,
            22 => Opcode::F,
            23 => Opcode::And,
            24 => Opcode::Or,
            25 => Opcode::Negate,
            26 => Opcode::Coalesce,
            27 => Opcode::LT,
            28 => Opcode::LTE,
            29 => Opcode::GT,
            30 => Opcode::GTE,
            31 => Opcode::Eq,
            32 => Opcode::Neq,
            33 => Opcode::OptMk,
            34 => Opcode::MapMk,
            35 => Opcode::MapLoad,
            36 => Opcode::ArrMk,
            37 => Opcode::ArrLoad,
            38 => Opcode::ArrSlc,
            39 => Opcode::GStore,
            40 => Opcode::LStore0,
            41 => Opcode::LStore1,
            42 => Opcode::LStore2,
            43 => Opcode::LStore3,
            44 => Opcode::LStore4,
            45 => Opcode::LStore,
            46 => Opcode::UStore0,
            47 => Opcode::UStore1,
            48 => Opcode::UStore2,
            49 => Opcode::UStore3,
            50 => Opcode::UStore4,
            51 => Opcode::UStore,
            52 => Opcode::GLoad,
            53 => Opcode::LLoad0,
            54 => Opcode::LLoad1,
            55 => Opcode::LLoad2,
            56 => Opcode::LLoad3,
            57 => Opcode::LLoad4,
            58 => Opcode::LLoad,
            59 => Opcode::ULoad0,
            60 => Opcode::ULoad1,
            61 => Opcode::ULoad2,
            62 => Opcode::ULoad3,
            63 => Opcode::ULoad4,
            64 => Opcode::ULoad,
            65 => Opcode::Jump,
            66 => Opcode::JumpIfF,
            67 => Opcode::JumpB,
            68 => Opcode::Invoke,
            69 => Opcode::ClosureMk,
            70 => Opcode::CloseUpvalue,
            71 => Opcode::CloseUpvalueAndPop,
            72 => Opcode::Pop,
            73 => Opcode::PopN,
            74 => Opcode::Return,
            _ => unreachable!()
        }
    }
}

impl Opcode {
    pub fn num_expected_imms(&self) -> u8 {
        match self {
            Opcode::Constant |
            Opcode::Jump |
            Opcode::JumpIfF |
            Opcode::JumpB |
            Opcode::ArrMk |
            Opcode::MapMk |
            Opcode::LStore |
            Opcode::UStore |
            Opcode::PopN |
            Opcode::LLoad |
            Opcode::ULoad => 1,
            Opcode::Invoke => 2,
            _ => 0
        }
    }
}
