#[derive(Clone, Copy, Debug)]
pub struct Instruction(u32);

impl Instruction {
    pub fn new(code: u32) -> Self {
        Self(code)
    }

    pub fn from_u32(code: u32) -> Self {
        Self(code)
    }

    pub fn to_u32(self) -> u32 {
        self.0
    }

    pub fn opcode(self) -> u32 {
        self.0 >> 28
    }

    pub fn a(self) -> usize {
        ((self.0 >> 6) & 7) as usize
    }

    pub fn b(self) -> usize {
        ((self.0 >> 3) & 7) as usize
    }

    pub fn c(self) -> usize {
        (self.0 & 7) as usize
    }

    pub fn imm_a(self) -> usize {
        ((self.0 >> 25) & 7) as usize
    }

    pub fn imm_value(self) -> u32 {
        self.0 & 0x1ffffff
    }

    pub fn parse(self) -> Option<ParsedInstruction> {
        ParsedInstruction::from_u32(self.0)
    }
}

pub enum ParsedInstruction {
    ConditionalMove { a: usize, b: usize, c: usize },
    ArrayIndex { a: usize, b: usize, c: usize },
    ArrayAmendment { a: usize, b: usize, c: usize },
    Addition { a: usize, b: usize, c: usize },
    Multiplication { a: usize, b: usize, c: usize },
    Division { a: usize, b: usize, c: usize },
    NotAnd { a: usize, b: usize, c: usize },
    Halt,
    Allocation { b: usize, c: usize },
    Abandonment { c: usize },
    Output { c: usize },
    Input { c: usize },
    LoadProgram { b: usize, c: usize },
    Immediate { a: usize, value: u32 },
}

impl ParsedInstruction {
    pub fn from_u32(code: u32) -> Option<ParsedInstruction> {
        match code >> 28 {
            0 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::ConditionalMove { a, b, c })
            }
            1 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::ArrayIndex { a, b, c })
            }
            2 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::ArrayAmendment { a, b, c })
            }
            3 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Addition { a, b, c })
            }
            4 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Multiplication { a, b, c })
            }
            5 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Division { a, b, c })
            }
            6 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::NotAnd { a, b, c })
            }
            7 => Some(ParsedInstruction::Halt),
            8 => {
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Allocation { b, c })
            }
            9 => {
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Abandonment { c })
            }
            10 => {
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Output { c })
            }
            11 => {
                let c = (code & 7) as usize;
                Some(ParsedInstruction::Input { c })
            }
            12 => {
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(ParsedInstruction::LoadProgram { b, c })
            }
            13 => {
                let a = ((code >> 25) & 7) as usize;
                let value = code & 0x1ffffff;
                Some(ParsedInstruction::Immediate { a, value })
            }
            _ => None,
        }
    }
}

impl std::fmt::Debug for ParsedInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConditionalMove { a, b, c } => write!(f, "cmove r{a}, r{b}, r{c}"),
            Self::ArrayIndex { a, b, c } => write!(f, "load r{a}, r{b}, r{c}"),
            Self::ArrayAmendment { a, b, c } => write!(f, "store r{a}, r{b}, r{c}"),
            Self::Addition { a, b, c } => write!(f, "add r{a}, r{b}, r{c}"),
            Self::Multiplication { a, b, c } => write!(f, "mul r{a}, r{b}, r{c}"),
            Self::Division { a, b, c } => write!(f, "div r{a} r{b}, r{c}"),
            Self::NotAnd { a, b, c } => write!(f, "nand r{a} r{b}, r{c}"),
            Self::Halt => write!(f, "halt"),
            Self::Allocation { b, c } => write!(f, "alloc r{b}, r{c}"),
            Self::Abandonment { c } => write!(f, "free r{c}"),
            Self::Output { c } => write!(f, "out r{c}"),
            Self::Input { c } => write!(f, "in r{c}"),
            Self::LoadProgram { b, c } => write!(f, "jmp r{b}, r{c}"),
            Self::Immediate { a, value } => write!(f, "imm r{a}, {}", value),
        }
    }
}
