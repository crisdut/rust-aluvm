// AluRE: AluVM runtime environment.
// This is rust implementation of AluVM (arithmetic logic unit virtual machine).
//
// Designed & written in 2021 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// This software is licensed under the terms of MIT License.
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use amplify::num::{u1024, u5, u512};
#[cfg(feature = "std")]
use std::fmt::{self, Display, Formatter};

use crate::registers::{Reg, Reg32, Reg8, RegA, RegR, Registers};
use crate::{LibSite, Value};

/// Turing machine movement after instruction execution
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ExecStep {
    /// Stop program execution
    Stop,

    /// Move to the next instruction
    Next,

    /// Jump to the offset from the origin
    Jump(u16),

    /// Jump to another code fragment
    Call(LibSite),
}

/// Trait for instructions
pub trait Instruction {
    /// Executes given instruction taking all registers as input and output.
    /// The method is provided with the current code position which may be
    /// used by the instruction for constructing call stack.
    ///
    /// Returns whether further execution should be stopped.
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep;

    /// Returns length of the instruction block in bytes
    fn len(self) -> u16;
}

/// Full set of instructions
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
// #[cfg_attr(feature = "std", derive(Display), display(inner))]
#[non_exhaustive]
pub enum Instr<Extension>
where
    Extension: Instruction,
{
    /// Control-flow instructions
    // #[value = 0b00_000_000]
    ControlFlow(ControlFlowOp),

    /// Instructions setting register values
    // #[value = 0b00_001_000]
    Put(PutOp),

    /// Instructions moving and swapping register values
    // #[value = 0b00_010_000]
    Move(MoveOp),

    /// Instructions comparing register values
    // #[value = 0b00_011_000]
    Cmp(CmpOp),

    /// Arithmetic instructions
    // #[value = 0b00_100_000]
    Arithmetic(ArithmeticOp),

    /// Bit operations & boolean algebra instructions
    // #[value = 0b00_101_000]
    Bitwise(BitwiseOp),

    /// Operations on byte strings
    // #[value = 0b00_110_000]
    Bytes(BytesOp),

    /// Cryptographic hashing functions
    // #[value = 0b01_000_000]
    Digest(DigestOp),

    /// Operations on Secp256k1 elliptic curve
    // #[value = 0b01_001_000]
    Secp256k1(SecpOp),

    /// Operations on Curve25519 elliptic curve
    // #[value = 0b01_001_100]
    Curve25519(Curve25519Op),

    /// Reserved operations which can be provided by a host environment
    // #[value = 0b10_000_000]
    ExtensionCodes(Extension),

    /// No-operation instruction
    // #[value = 0b11_111_111]
    Nop,
}

impl<Extension> Instruction for Instr<Extension>
where
    Extension: Instruction,
{
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        match self {
            Instr::ControlFlow(instr) => instr.exec(regs, site),
            Instr::Put(instr) => instr.exec(regs, site),
            Instr::Move(instr) => instr.exec(regs, site),
            Instr::Cmp(instr) => instr.exec(regs, site),
            Instr::Arithmetic(instr) => instr.exec(regs, site),
            Instr::Bitwise(instr) => instr.exec(regs, site),
            Instr::Bytes(instr) => instr.exec(regs, site),
            Instr::Digest(instr) => instr.exec(regs, site),
            Instr::Secp256k1(instr) => instr.exec(regs, site),
            Instr::Curve25519(instr) => instr.exec(regs, site),
            Instr::ExtensionCodes(instr) => instr.exec(regs, site),
            Instr::Nop => ExecStep::Next,
        }
    }

    fn len(self) -> u16 {
        match self {
            Instr::ControlFlow(instr) => instr.len(),
            Instr::Put(instr) => instr.len(),
            Instr::Move(instr) => instr.len(),
            Instr::Cmp(instr) => instr.len(),
            Instr::Arithmetic(instr) => instr.len(),
            Instr::Bitwise(instr) => instr.len(),
            Instr::Bytes(instr) => instr.len(),
            Instr::Digest(instr) => instr.len(),
            Instr::Secp256k1(instr) => instr.len(),
            Instr::Curve25519(instr) => instr.len(),
            Instr::ExtensionCodes(instr) => instr.len(),
            Instr::Nop => 1,
        }
    }
}

/// Control-flow instructions
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "std", derive(Display))]
pub enum ControlFlowOp {
    /// Completes program execution writing `false` to `st0` (indicating
    /// program failure)
    #[cfg_attr(feature = "std", display("fail"))]
    // #[value = 0b000]
    Fail,

    /// Completes program execution writing `true` to `st0` (indicating program
    /// success)
    #[cfg_attr(feature = "std", display("succ"))]
    // #[value = 0b001]
    Succ,

    /// Unconditionally jumps to an offset. Increments `cy0`.
    #[cfg_attr(feature = "std", display("jmp\t{0:#06X}"))]
    // #[value = 0b010]
    Jmp(u16),

    /// Jumps to an offset if `st0` == true, otherwise does nothing. Increments
    /// `cy0`.
    #[cfg_attr(feature = "std", display("jif\t{0:#06X}"))]
    // #[value = 0b011]
    Jif(u16),

    /// Jumps to other location in the current code with ability to return
    /// back (calls a subroutine). Increments `cy0` and pushes offset of the
    /// instruction which follows current one to `cs0`.
    #[cfg_attr(feature = "std", display("routine\t{0:#06X}"))]
    Routine(u16),

    /// Calls code from an external library identified by the hash of its code.
    /// Increments `cy0` and `cp0` and pushes offset of the instruction which
    /// follows current one to `cs0`.
    #[cfg_attr(feature = "std", display("call\t{0}"))]
    Call(LibSite),

    /// Passes execution to other library without an option to return.
    /// Does not increments `cy0` and `cp0` counters and does not add anything
    /// to the call stack `cs0`.
    #[cfg_attr(feature = "std", display("exec\t{0}"))]
    Exec(LibSite),

    /// Returns execution flow to the previous location from the top of `cs0`.
    /// Does not change value in `cy0`. Decrements `cp0`.
    #[cfg_attr(feature = "std", display("ret"))]
    Ret,
}

impl Instruction for ControlFlowOp {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        match self {
            ControlFlowOp::Fail => {
                regs.st0 = false;
                ExecStep::Stop
            }
            ControlFlowOp::Succ => {
                regs.st0 = true;
                ExecStep::Stop
            }
            ControlFlowOp::Jmp(offset) => regs
                .jmp()
                .map(|_| ExecStep::Jump(offset))
                .unwrap_or(ExecStep::Stop),
            ControlFlowOp::Jif(offset) => {
                if regs.st0 == true {
                    regs.jmp()
                        .map(|_| ExecStep::Jump(offset))
                        .unwrap_or(ExecStep::Stop)
                } else {
                    ExecStep::Next
                }
            }
            ControlFlowOp::Routine(offset) => regs
                .call(site)
                .map(|_| ExecStep::Jump(offset))
                .unwrap_or(ExecStep::Stop),
            ControlFlowOp::Call(site) => regs
                .call(site)
                .map(|_| ExecStep::Call(site))
                .unwrap_or(ExecStep::Stop),
            ControlFlowOp::Exec(site) => regs
                .jmp()
                .map(|_| ExecStep::Call(site))
                .unwrap_or(ExecStep::Stop),
            ControlFlowOp::Ret => {
                regs.ret().map(ExecStep::Call).unwrap_or(ExecStep::Stop)
            }
        }
    }

    fn len(self) -> u16 {
        match self {
            ControlFlowOp::Fail => 1,
            ControlFlowOp::Succ => 1,
            ControlFlowOp::Jmp(_) => 3,
            ControlFlowOp::Jif(_) => 3,
            ControlFlowOp::Routine(_) => 3,
            ControlFlowOp::Call(_) => 3 + 32,
            ControlFlowOp::Exec(_) => 3 + 32,
            ControlFlowOp::Ret => 1,
        }
    }
}

/// Instructions setting register values
#[cfg_attr(feature = "std", derive(Display))]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PutOp {
    /// Sets `a` register value to zero
    #[cfg_attr(feature = "std", display("zero\t{0}{1}"))]
    ZeroA(RegA, Reg32),

    /// Sets `r` register value to zero
    #[cfg_attr(feature = "std", display("zero\t{0}{1}"))]
    ZeroR(RegR, Reg32),

    /// Cleans a value of `a` register (sets it to undefined state)
    #[cfg_attr(feature = "std", display("cl\t{0}{1}"))]
    ClA(RegA, Reg32),

    /// Cleans a value of `r` register (sets it to undefined state)
    #[cfg_attr(feature = "std", display("cl\t{0}{1}"))]
    ClR(RegR, Reg32),

    /// Unconditionally assigns a value to `a` register
    #[cfg_attr(feature = "std", display("put\t{0}{1}, {2}"))]
    PutA(RegA, Reg32, Value),

    /// Unconditionally assigns a value to `r` register
    #[cfg_attr(feature = "std", display("put\t{0}{1}, {2}"))]
    PutR(RegR, Reg32, Value),

    /// Conditionally assigns a value to `a` register if the register is in
    /// uninitialized state
    #[cfg_attr(feature = "std", display("putif\t{0}{1}, {2}"))]
    PutAIf(RegA, Reg32, Value),

    /// Conditionally assigns a value to `r` register if the register is in
    /// uninitialized state
    #[cfg_attr(feature = "std", display("putif\t{0}{1}, {2}"))]
    PutRIf(RegR, Reg32, Value),
}

impl Instruction for PutOp {
    fn exec(self, regs: &mut Registers, _: LibSite) -> ExecStep {
        match self {
            PutOp::ZeroA(reg, index) => {
                regs.set(Reg::A(reg), index, Some(0.into()))
            }
            PutOp::ZeroR(reg, index) => {
                regs.set(Reg::R(reg), index, Some(0.into()))
            }
            PutOp::ClA(reg, index) => regs.set(Reg::A(reg), index, None),
            PutOp::ClR(reg, index) => regs.set(Reg::R(reg), index, None),
            PutOp::PutA(reg, index, blob) => {
                regs.set(Reg::A(reg), index, Some(blob))
            }
            PutOp::PutR(reg, index, blob) => {
                regs.set(Reg::R(reg), index, Some(blob))
            }
            PutOp::PutAIf(reg, index, blob) => {
                regs.get(Reg::A(reg), index).or_else(|| {
                    regs.set(Reg::A(reg), index, Some(blob));
                    Some(blob)
                });
            }
            PutOp::PutRIf(reg, index, blob) => {
                regs.get(Reg::R(reg), index).or_else(|| {
                    regs.set(Reg::R(reg), index, Some(blob));
                    Some(blob)
                });
            }
        }
        ExecStep::Next
    }

    fn len(self) -> u16 {
        match self {
            PutOp::ZeroA(_, _)
            | PutOp::ZeroR(_, _)
            | PutOp::ClA(_, _)
            | PutOp::ClR(_, _) => 2,
            PutOp::PutA(_, _, Value { len, .. })
            | PutOp::PutR(_, _, Value { len, .. })
            | PutOp::PutAIf(_, _, Value { len, .. })
            | PutOp::PutRIf(_, _, Value { len, .. }) => {
                4u16.saturating_add(len)
            }
        }
    }
}

/// Integer arithmetic types
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "std", derive(Display))]
pub enum NumType {
    /// Unsigned integer
    #[cfg_attr(feature = "std", display("u"))]
    Unsigned,

    /// Signed integer
    #[cfg_attr(feature = "std", display("s"))]
    Signed,

    /// Float number with 23-bit mantissa
    #[cfg_attr(feature = "std", display("f"))]
    Float23,

    /// Float number with 52 bit mantissa
    #[cfg_attr(feature = "std", display("d"))]
    Float52,
}

/// Instructions moving and swapping register values
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "std", derive(Display))]
pub enum MoveOp {
    /// Swap operation for arithmetic registers. If the value does not fit
    /// destination bit dimensions truncates the most significant bits until
    /// they fit.
    #[cfg_attr(feature = "std", display("swp\t{0}{1},{2}{3}"))]
    SwpA(RegA, Reg32, RegA, Reg32),

    /// Swap operation for non-arithmetic registers. If the value does not fit
    /// destination bit dimensions truncates the most significant bits until
    /// they fit.
    #[cfg_attr(feature = "std", display("swp\t{0}{1},{2}{3}"))]
    SwpR(RegR, Reg32, RegR, Reg32),

    /// Swap operation between arithmetic and non-arithmetic registers. If the
    /// value does not fit destination bit dimensions truncates the most
    /// significant bits until they fit.
    #[cfg_attr(feature = "std", display("swp\t{0}{1},{2}{3}"))]
    Swp(RegA, Reg32, RegR, Reg32),

    /// Array move operation: duplicates values of all register set into
    /// another set
    #[cfg_attr(feature = "std", display("amov{2}\t{0},{1}"))]
    AMov(RegA, RegA, NumType),

    /// Move operation: duplicates value of one of the arithmetic registers
    /// into another arithmetic register
    #[cfg_attr(feature = "std", display("mov\t{0}{1},{2}{3}"))]
    MovA(RegA, Reg32, RegA, Reg32),

    /// Move operation: duplicates value of one of the non-arithmetic registers
    /// into another non-arithmetic register
    #[cfg_attr(feature = "std", display("mov\t{0}{1},{2}{3}"))]
    MovR(RegR, Reg32, RegR, Reg32),

    /// Move operation: duplicates value of one of the arithmetic registers
    /// into non-arithmetic register
    #[cfg_attr(feature = "std", display("mov\t{0}{1},{2}{3}"))]
    MovAR(RegA, Reg32, RegR, Reg32),

    /// Move operation: duplicates value of one of the n on-arithmetic
    /// registers into arithmetic register
    #[cfg_attr(feature = "std", display("mov\t{0}{1},{2}{3}"))]
    MovRA(RegR, Reg32, RegA, Reg32),
}

impl Instruction for MoveOp {
    fn exec(self, regs: &mut Registers, _: LibSite) -> ExecStep {
        match self {
            MoveOp::SwpA(reg1, index1, reg2, index2) => {
                let val1 = regs.get(Reg::A(reg1), index1);
                let val2 = regs.get(Reg::A(reg2), index2);
                regs.set(Reg::A(reg1), index1, val2);
                regs.set(Reg::A(reg2), index2, val1);
            }
            MoveOp::SwpR(reg1, index1, reg2, index2) => {
                let val1 = regs.get(Reg::R(reg1), index1);
                let val2 = regs.get(Reg::R(reg2), index2);
                regs.set(Reg::R(reg1), index1, val2);
                regs.set(Reg::R(reg2), index2, val1);
            }
            MoveOp::Swp(reg1, index1, reg2, index2) => {
                let val1 = regs.get(Reg::A(reg1), index1);
                let val2 = regs.get(Reg::R(reg2), index2);
                regs.set(Reg::A(reg1), index1, val2);
                regs.set(Reg::R(reg2), index2, val1);
            }
            MoveOp::AMov(reg1, reg2, ty) => {
                todo!("Array move operation")
            }
            MoveOp::MovA(sreg, sidx, dreg, didx) => {
                regs.set(Reg::A(dreg), didx, regs.get(Reg::A(sreg), sidx));
            }
            MoveOp::MovR(sreg, sidx, dreg, didx) => {
                regs.set(Reg::R(dreg), didx, regs.get(Reg::R(sreg), sidx));
            }
            MoveOp::MovAR(sreg, sidx, dreg, didx) => {
                regs.set(Reg::R(dreg), didx, regs.get(Reg::A(sreg), sidx));
            }
            MoveOp::MovRA(sreg, sidx, dreg, didx) => {
                regs.set(Reg::A(dreg), didx, regs.get(Reg::R(sreg), sidx));
            }
        }
        ExecStep::Next
    }

    fn len(self) -> u16 {
        match self {
            MoveOp::SwpA(_, _, _, _)
            | MoveOp::SwpR(_, _, _, _)
            | MoveOp::Swp(_, _, _, _) => 3,
            MoveOp::AMov(_, _, _) => 2,
            MoveOp::MovA(_, _, _, _)
            | MoveOp::MovR(_, _, _, _)
            | MoveOp::MovAR(_, _, _, _)
            | MoveOp::MovRA(_, _, _, _) => 3,
        }
    }
}

/// Instructions comparing register values
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "std", derive(Display))]
pub enum CmpOp {
    /// Compares value of two arithmetic (`A`) registers setting `st0` to
    /// `true` if the first parameter is greater (and not equal) than the
    /// second one
    // #[value = 0b110] // 3 + 5 + 3 + 5 => 16 bits
    #[cfg_attr(feature = "std", display("gt\t{0}{1},{2}{3}"))]
    Gt(RegA, Reg32, RegA, Reg32),

    /// Compares value of two non-arithmetic (`R`) registers setting `st0` to
    /// `true` if the first parameter is less (and not equal) than the second
    /// one
    // #[value = 0b111]
    #[cfg_attr(feature = "std", display("lt\t{0}{1},{2}{3}"))]
    Lt(RegR, Reg32, RegR, Reg32),

    /// Checks equality of value in two arithmetic (`A`) registers putting
    /// result into `st0`
    // #[value = 0b100]
    #[cfg_attr(feature = "std", display("eq\t{0}{1},{2}{3}"))]
    Eqa(RegA, Reg32, RegA, Reg32),

    /// Checks equality of value in two non-arithmetic (`R`) registers putting
    /// result into `st0`
    // #[value = 0b101]
    #[cfg_attr(feature = "std", display("eq\t{0}{1},{2}{3}"))]
    Eqr(RegR, Reg32, RegR, Reg32),

    /// Measures bit length of a value in one fo the registers putting result
    /// to `a16[0]`
    #[cfg_attr(feature = "std", display("len\t{0}{1}"))]
    Len(RegA, Reg32),

    /// Counts number of `1` bits in register putting result to `a16[0]`
    /// register.
    #[cfg_attr(feature = "std", display("cnt\t{0}{1}"))]
    Cnt(RegA, Reg32),

    /// Assigns value of `a8[0]` register to `st0`
    #[cfg_attr(feature = "std", display("st2a"))]
    St2A,

    /// `st0` value of `st0` register to the result of `a8[0] == 1`
    #[cfg_attr(feature = "std", display("a2st"))]
    A2St,
}

impl Instruction for CmpOp {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        todo!()
    }

    fn len(self) -> u16 {
        match self {
            CmpOp::Gt(_, _, _, _)
            | CmpOp::Lt(_, _, _, _)
            | CmpOp::Eqa(_, _, _, _)
            | CmpOp::Eqr(_, _, _, _) => 3,
            CmpOp::Len(_, _) | CmpOp::Cnt(_, _) => 2,
            CmpOp::St2A | CmpOp::A2St => 1,
        }
    }
}

/// Variants of arithmetic operations
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Arithmetics {
    IntChecked {
        /// Indicates the need to use signed integer arithmetics
        signed: bool,
    },
    IntUnchecked {
        /// Indicates the need to use signed integer arithmetics
        signed: bool,
    },
    IntArbitraryPrecision {
        /// Indicates the need to use signed integer arithmetics
        signed: bool,
    },
    Float,
    FloatArbitraryPrecision,
}

#[cfg(feature = "std")]
impl Display for Arithmetics {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Arithmetics::IntChecked { signed: false } => f.write_str("c"),
            Arithmetics::IntUnchecked { signed: false } => f.write_str(""),
            Arithmetics::IntArbitraryPrecision { signed: false } => {
                f.write_str("a")
            }
            Arithmetics::IntChecked { signed: true } => f.write_str("cs"),
            Arithmetics::IntUnchecked { signed: true } => f.write_str("s"),
            Arithmetics::IntArbitraryPrecision { signed: true } => {
                f.write_str("as")
            }
            Arithmetics::Float => f.write_str("f"),
            Arithmetics::FloatArbitraryPrecision => f.write_str("af"),
        }
    }
}

/// Arithmetic instructions
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "std", derive(Display))]
pub enum ArithmeticOp {
    /// Negates most significant bit
    #[cfg_attr(feature = "std", display("neg\t{0}{1}"))]
    Neg(RegA, Reg32),

    /// Increases register value on a given step.
    #[cfg_attr(feature = "std", display("add{0}\t{1}{2},{3}"))]
    Inc(Arithmetics, RegA, Reg32, u5),

    /// Adds two registers. Puts result to `a_[0]` or `ap[0]`, if
    /// [`Arithmetics::IntArbitraryPrecision`] or
    /// [`Arithmetics::FloatArbitraryPrecision`] is used
    #[cfg_attr(feature = "std", display("add{0}\t{1}{2},{1}{3}"))]
    Add(Arithmetics, RegA, Reg32, Reg32),

    /// Subtracts two registers. Puts result to `a_[0]` or `ap[0]`, if
    /// [`Arithmetics::IntArbitraryPrecision`] or
    /// [`Arithmetics::FloatArbitraryPrecision`] is used
    #[cfg_attr(feature = "std", display("sub{0}\t{1}{2},{1}{3}"))]
    Sub(Arithmetics, RegA, Reg32, Reg32),

    /// Multiplies two registers. Puts result to `a_[0]` or `ap[0]`, if
    /// [`Arithmetics::IntArbitraryPrecision`] or
    /// [`Arithmetics::FloatArbitraryPrecision`] is used
    #[cfg_attr(feature = "std", display("mul{0}\t{1}{2},{1}{3}"))]
    Mul(Arithmetics, RegA, Reg32, Reg32),

    /// Divides two registers. Puts result to `a_[0]` or `ap[0]`, if
    /// [`Arithmetics::IntArbitraryPrecision`] or
    /// [`Arithmetics::FloatArbitraryPrecision`] is used
    #[cfg_attr(feature = "std", display("div{0}\t{1}{2},{1}{3}"))]
    Div(Arithmetics, RegA, Reg32, Reg32),

    /// Modulo division
    #[cfg_attr(feature = "std", display("mod\t{0}{1},{2}{3},{4}{5}"))]
    Mod(RegA, Reg32, RegA, Reg32, RegA, Reg32),

    /// Puts absolute value of register into `a8[0]`
    #[cfg_attr(feature = "std", display("abs\t{0}{1}"))]
    Abs(RegA, Reg32),
}

impl Instruction for ArithmeticOp {
    fn exec(self, regs: &mut Registers, _: LibSite) -> ExecStep {
        match self {
            ArithmeticOp::Neg(reg, index) => {
                regs.get(Reg::A(reg), index).map(|mut blob| {
                    blob.bytes[reg as usize] = 0xFF ^ blob.bytes[reg as usize];
                    regs.set(Reg::A(reg), index, Some(blob));
                });
            }
            ArithmeticOp::Inc(arithm, reg, index, step) => {
                regs.get(Reg::A(reg), index).map(|value| {
                    let u512_max = u512::from_le_bytes([0xFF; 64]);
                    let res = match arithm {
                        Arithmetics::IntChecked { signed: false } => {
                            let step = u512::from_u64(*step as u64).unwrap();
                            let mut val: u512 = value.into();
                            if step >= u512_max - val {
                                None
                            } else {
                                val = val + step;
                                Some(Value::from(val))
                            }
                        }
                        Arithmetics::IntUnchecked { signed: false } => {
                            let step = u512::from_u64(*step as u64).unwrap();
                            let mut val: u512 = value.into();
                            if step >= u512_max - val {
                                Some(Value::from(step - (u512_max - val)))
                            } else {
                                val = val + step;
                                Some(Value::from(val))
                            }
                        }
                        Arithmetics::IntArbitraryPrecision {
                            signed: false,
                        } => {
                            todo!("Arbitrary precision increment")
                        }
                        Arithmetics::IntChecked { signed: true } => {
                            todo!("Signed increment")
                        }
                        Arithmetics::IntUnchecked { signed: true } => {
                            todo!("Signed increment")
                        }
                        Arithmetics::IntArbitraryPrecision { signed: true } => {
                            todo!("Arbitrary precision signed increment")
                        }
                        Arithmetics::Float => todo!("Float increment"),
                        Arithmetics::FloatArbitraryPrecision => {
                            todo!("Float increment")
                        }
                    };
                    regs.set(Reg::A(reg), index, res);
                });
            }
            ArithmeticOp::Add(arithm, reg, src, dst) => {
                regs.get(Reg::A(reg), src).and_then(|value1| {
                    regs.get(Reg::A(reg), dst).map(|value2| (value1, value2))
                }).map(|(value1, value2)| {
                    let mut dst_reg = Reg::A(reg);
                    let res = match arithm {
                        Arithmetics::IntChecked { signed: false } => {
                            // TODO: Support source arbitrary precision registers
                            let mut val: u1024 = value1.into();
                            val = val + u1024::from(value2);
                            Value::from(val)
                        }
                        Arithmetics::IntUnchecked { signed: false } => {
                            // TODO: Support source arbitrary precision registers
                            let mut val: u1024 = value1.into();
                            val = val + u1024::from(value2);
                            Value::from(val)
                        }
                        Arithmetics::IntArbitraryPrecision {
                            signed: false,
                        } => {
                            dst_reg = Reg::A(RegA::AP);
                            todo!("Unsigned int addition with arbitrary precision")
                        }
                        Arithmetics::IntChecked { signed: true } => todo!("Signed int addition"),
                        Arithmetics::IntUnchecked { signed: true } => todo!("Signed int addition"),
                        Arithmetics::IntArbitraryPrecision { signed: true } => {
                            dst_reg = Reg::A(RegA::AP);
                            todo!("Signed int addition with arbitrary precision")
                        }
                        Arithmetics::Float => todo!("Float addition"),
                        Arithmetics::FloatArbitraryPrecision => {
                            dst_reg = Reg::A(RegA::AP);
                            todo!("Float addition with arbitrary precision")
                        }
                    };
                    regs.set(dst_reg, Reg32::Reg1, Some(res));
                });
            }
            ArithmeticOp::Sub(arithm, reg, src, dst) => {}
            ArithmeticOp::Mul(arithm, reg, src, dst) => {
                regs.get(Reg::A(reg), src).and_then(|value1| {
                    regs.get(Reg::A(reg), dst).map(|value2| (value1, value2))
                }).map(|(value1, value2)| {
                    let mut dst_reg = Reg::A(reg);
                    let res = match arithm {
                        Arithmetics::IntChecked { signed: false } => {
                            // TODO: Rewrite
                            let mut val: u1024 = value1.into();
                            val = val * u1024::from(value2);
                            Value::from(val)
                        }
                        Arithmetics::IntUnchecked { signed: false } => {
                            // TODO: Rewrite
                            let mut val: u1024 = value1.into();
                            val = val * u1024::from(value2);
                            Value::from(val)
                        }
                        Arithmetics::IntArbitraryPrecision {
                            signed: false,
                        } => {
                            dst_reg = Reg::A(RegA::AP);
                            todo!("Unsigned int multiplication with arbitrary precision")
                        }
                        Arithmetics::IntChecked { signed: true } => todo!("Signed int multiplication"),
                        Arithmetics::IntUnchecked { signed: true } => todo!("Signed int multiplication"),
                        Arithmetics::IntArbitraryPrecision { signed: true } => {
                            dst_reg = Reg::A(RegA::AP);
                            todo!("Signed int multiplication with arbitrary precision")
                        }
                        Arithmetics::Float => todo!("Float addition"),
                        Arithmetics::FloatArbitraryPrecision => {
                            dst_reg = Reg::A(RegA::AP);
                            todo!("Float multiplication with arbitrary precision")
                        }
                    };
                    regs.set(dst_reg, Reg32::Reg1, Some(res));
                });
            }
            ArithmeticOp::Div(arithm, reg, src, dst) => {}
            ArithmeticOp::Mod(reg1, index1, reg2, index2, reg3, index3) => {}
            ArithmeticOp::Abs(reg, index) => {}
        }
        ExecStep::Next
    }

    fn len(self) -> u16 {
        match self {
            ArithmeticOp::Neg(_, _) => 2,
            ArithmeticOp::Inc(_, _, _, _) => 3,
            ArithmeticOp::Add(_, _, _, _)
            | ArithmeticOp::Sub(_, _, _, _)
            | ArithmeticOp::Mul(_, _, _, _)
            | ArithmeticOp::Div(_, _, _, _) => 3,
            ArithmeticOp::Mod(_, _, _, _, _, _) => 4,
            ArithmeticOp::Abs(_, _) => 2,
        }
    }
}

/// Bit operations & boolean algebra instructions
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "std", derive(Display))]
pub enum BitwiseOp {
    /// Bitwise AND operation
    #[cfg_attr(feature = "std", display("and\t{0}{1},{0}{2},{0}{3}"))]
    And(
        RegA,
        Reg32,
        Reg32,
        /// Operation destination, only first 8 registers
        Reg8,
    ),

    /// Bitwise OR operation
    #[cfg_attr(feature = "std", display("or\t{0}{1},{0}{2},{0}{3}"))]
    Or(RegA, Reg32, Reg32, Reg8),

    /// Bitwise XOR operation
    #[cfg_attr(feature = "std", display("xor\t{0}{1},{0}{2},{0}{3}"))]
    Xor(RegA, Reg32, Reg32, Reg8),

    /// Bitwise inversion
    #[cfg_attr(feature = "std", display("not\t{0}{1}"))]
    Not(RegA, Reg32),

    /// Left bit shift, filling added bits values with zeros
    #[cfg_attr(feature = "std", display("shl\t{0}{1},a8{2},{0}{3}"))]
    Shl(RegA, Reg32, Reg32 /* Always `a8` */, Reg8),

    /// Right bit shift, filling added bits values with zeros
    #[cfg_attr(feature = "std", display("shr\t{0}{1},a8{2},{0}{3}"))]
    Shr(RegA, Reg32, Reg32, Reg8),

    /// Left bit shift, cycling the shifted values (most significant bit
    /// becomes least significant)
    #[cfg_attr(feature = "std", display("scl\t{0}{1},a8{2},{0}{3}"))]
    Scl(RegA, Reg32, Reg32, Reg8),

    /// Right bit shift, cycling the shifted values (least significant bit
    /// becomes nost significant)
    #[cfg_attr(feature = "std", display("scr\t{0}{1},a8{2},{0}{3}"))]
    Scr(RegA, Reg32, Reg32, Reg8),
}

impl Instruction for BitwiseOp {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        todo!()
    }

    fn len(self) -> u16 {
        match self {
            BitwiseOp::And(_, _, _, _)
            | BitwiseOp::Or(_, _, _, _)
            | BitwiseOp::Xor(_, _, _, _) => 3,
            BitwiseOp::Not(_, _) => 2,
            BitwiseOp::Shl(_, _, _, _)
            | BitwiseOp::Shr(_, _, _, _)
            | BitwiseOp::Scl(_, _, _, _)
            | BitwiseOp::Scr(_, _, _, _) => 3,
        }
    }
}

/// Operations on byte strings
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum BytesOp {
    Puts(/** `s` register index */ u8, u16, [u8; u16::MAX as usize]),

    Movs(/** `s` register index */ u8, /** `s` register index */ u8),

    Swps(/** `s` register index */ u8, /** `s` register index */ u8),

    Fill(
        /** `s` register index */ u8,
        /** from */ u16,
        /** to */ u16,
        /** value */ u8,
    ),

    /// Returns length of the string
    Lens(/** `s` register index */ u8),

    /// Counts number of byte occurrences within the string
    Counts(/** `s` register index */ u8, /** byte to count */ u8),

    /// Compares two strings from two registers, putting result into `cm0`
    Cmps(u8, u8),

    /// Computes length of the fragment shared between two strings
    Common(u8, u8),

    /// Counts number of occurrences of one string within another putting
    /// result to `a16[0]`
    Find(
        /** `s` register with string */ u8,
        /** `s` register with matching fragment */ u8,
    ),

    /// Extracts value into a register
    Exta(RegA, Reg32, /** `s` register index */ u8, /** offset */ u16),
    Extr(RegR, Reg32, /** `s` register index */ u8, /** offset */ u16),

    Join(
        /** Source 1 */ u8,
        /** Source 2 */ u8,
        /** Destination */ u8,
    ),
    Split(
        /** Source */ u8,
        /** Offset */ u16,
        /** Destination 1 */ u8,
        /** Destination 2 */ u8,
    ),
    Ins(
        /** Insert from register */ u8,
        /** Insert to register */ u8,
        /** Offset for insert place */ u16,
    ),
    Del(
        /** Register index */ u8,
        /** Delete from */ u16,
        /** Delete to */ u16,
    ),
    /// Translocates fragment of bytestring into a register
    Transl(
        /** Source */ u8,
        /** Start from */ u16,
        /** End at */ u16,
        /** Index to put translocated portion */ u8,
    ),
}

impl Instruction for BytesOp {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        todo!()
    }

    fn len(self) -> u16 {
        match self {
            BytesOp::Puts(_, len, _) => 4u16.saturating_add(len),
            BytesOp::Movs(_, _) | BytesOp::Swps(_, _) => 3,
            BytesOp::Fill(_, _, _, _) => 7,
            BytesOp::Lens(_) => 2,
            BytesOp::Counts(_, _) => 3,
            BytesOp::Cmps(_, _) => 3,
            BytesOp::Common(_, _) => 3,
            BytesOp::Find(_, _) => 3,
            BytesOp::Exta(_, _, _, _) | BytesOp::Extr(_, _, _, _) => 4,
            BytesOp::Join(_, _, _) => 4,
            BytesOp::Split(_, _, _, _) => 6,
            BytesOp::Ins(_, _, _) | BytesOp::Del(_, _, _) => 5,
            BytesOp::Transl(_, _, _, _) => 7,
        }
    }
}

/// Cryptographic hashing functions
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum DigestOp {
    Ripemd(
        /** Which of `a16` registers contain start offset */ Reg32,
        /** Index of string register */ Reg32,
        /** Index of `r160` register to save result to */ Reg32,
        /** Clear string register after operation */ bool,
    ),
    Sha2(
        /** Which of `a16` registers contain start offset */ Reg32,
        /** Index of string register */ Reg32,
        /** Index of `r160` register to save result to */ Reg32,
        /** Clear string register after operation */ bool,
    ),
}

impl Instruction for DigestOp {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        todo!()
    }

    fn len(self) -> u16 {
        3
    }
}

/// Operations on Secp256k1 elliptic curve
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SecpOp {
    Gen(
        /** Register containing scalar */ Reg32,
        /** Destination register to put G * scalar */ Reg8,
    ),
    Mul(
        /** Use `a` or `r` register as scalar source */ bool,
        /** Scalar register index */ Reg32,
        /** Source `r` register index containing EC point */ Reg32,
        /** Destination `r` register index */ Reg32,
    ),
    Add(
        /** Allow overflows */ bool,
        /** Source 1 */ Reg32,
        /** Source 2 */ Reg32,
        /** Source 3 */ Reg32,
    ),
    Neg(
        /** Register hilding EC point to negate */ Reg32,
        /** Destination register */ Reg8,
    ),
}

impl Instruction for SecpOp {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        todo!()
    }

    fn len(self) -> u16 {
        match self {
            SecpOp::Gen(_, _) => 2,
            SecpOp::Mul(_, _, _, _) => 3,
            SecpOp::Add(_, _, _, _) => 3,
            SecpOp::Neg(_, _) => 2,
        }
    }
}

/// Operations on Curve25519 elliptic curve
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Curve25519Op {
    Gen(
        /** Register containing scalar */ Reg32,
        /** Destination register to put G * scalar */ Reg8,
    ),
    Mul(
        /** Use `a` or `r` register as scalar source */ bool,
        /** Scalar register index */ Reg32,
        /** Source `r` register index containing EC point */ Reg32,
        /** Destination `r` register index */ Reg32,
    ),
    Add(
        /** Allow overflows */ bool,
        /** Source 1 */ Reg32,
        /** Source 2 */ Reg32,
        /** Source 3 */ Reg32,
    ),
    Neg(
        /** Register hilding EC point to negate */ Reg32,
        /** Destination register */ Reg8,
    ),
}

impl Instruction for Curve25519Op {
    fn exec(self, regs: &mut Registers, site: LibSite) -> ExecStep {
        todo!()
    }

    fn len(self) -> u16 {
        match self {
            Curve25519Op::Gen(_, _) => 2,
            Curve25519Op::Mul(_, _, _, _) => 3,
            Curve25519Op::Add(_, _, _, _) => 3,
            Curve25519Op::Neg(_, _) => 2,
        }
    }
}
