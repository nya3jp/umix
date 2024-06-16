pub enum Instruction {
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

impl Instruction {
    pub fn from_u32(code: u32) -> Option<Instruction> {
        match code >> 28 {
            0 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::ConditionalMove { a, b, c })
            }
            1 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::ArrayIndex { a, b, c })
            }
            2 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::ArrayAmendment { a, b, c })
            }
            3 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::Addition { a, b, c })
            }
            4 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::Multiplication { a, b, c })
            }
            5 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::Division { a, b, c })
            }
            6 => {
                let a = ((code >> 6) & 7) as usize;
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::NotAnd { a, b, c })
            }
            7 => Some(Instruction::Halt),
            8 => {
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::Allocation { b, c })
            }
            9 => {
                let c = (code & 7) as usize;
                Some(Instruction::Abandonment { c })
            }
            10 => {
                let c = (code & 7) as usize;
                Some(Instruction::Output { c })
            }
            11 => {
                let c = (code & 7) as usize;
                Some(Instruction::Input { c })
            }
            12 => {
                let b = ((code >> 3) & 7) as usize;
                let c = (code & 7) as usize;
                Some(Instruction::LoadProgram { b, c })
            }
            13 => {
                let a = ((code >> 25) & 7) as usize;
                let value = code & 0x1ffffff;
                Some(Instruction::Immediate { a, value })
            }
            _ => None,
        }
    }
}

impl std::fmt::Debug for Instruction {
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
