use super::{Instruction, InstructionNode, Variant};
use crate::common::*;
use crate::linter::*;
use crate::parser::*;

pub struct InstructionContext {
    pub instructions: Vec<InstructionNode>,
    pub constants: Vec<CaseInsensitiveString>,
}

impl InstructionContext {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
            constants: vec![],
        }
    }

    pub fn push(&mut self, i: Instruction, pos: Location) {
        self.instructions.push(i.at(pos));
    }

    pub fn jump_if_false<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::UnresolvedJumpIfFalse(CaseInsensitiveString::new(format!(
                "_{}_{:?}",
                prefix.as_ref(),
                pos
            ))),
            pos,
        );
    }

    pub fn jump<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::UnresolvedJump(CaseInsensitiveString::new(format!(
                "_{}_{:?}",
                prefix.as_ref(),
                pos
            ))),
            pos,
        );
    }

    pub fn label<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::Label(CaseInsensitiveString::new(format!(
                "_{}_{:?}",
                prefix.as_ref(),
                pos
            ))),
            pos,
        );
    }

    pub fn store_temp_var<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::Store(Name::Bare(CaseInsensitiveString::new(format!(
                "{}{:?}",
                prefix.as_ref(),
                pos
            )))),
            pos,
        );
    }

    pub fn copy_temp_var_to_a<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::CopyVarToA(Name::Bare(CaseInsensitiveString::new(format!(
                "{}{:?}",
                prefix.as_ref(),
                pos
            )))),
            pos,
        );
    }

    pub fn copy_temp_var_to_b<S: AsRef<str>>(&mut self, prefix: S, pos: Location) {
        self.push(
            Instruction::CopyVarToB(Name::Bare(CaseInsensitiveString::new(format!(
                "{}{:?}",
                prefix.as_ref(),
                pos
            )))),
            pos,
        );
    }
}

// impl Visitor<QStatementNode> for Emitter {
//     fn visit(&mut self, a: &QStatementNode) -> Result<()> {
//         match a {
//             QStatementNode::Assignment(l, r) => {
//                 self.visit(r)?;
//                 self.push(Instruction::Store(l.as_ref().clone()), l.location());
//                 Ok(())
//             }
//             _ => unimplemented!(),
//         }
//     }
// }

// impl Visitor<QExpressionNode> for Emitter {
//     fn visit(&mut self, a: &QExpressionNode) -> Result<()> {
//         match a {

//         }
//     }
// }

// impl Visitor<QProgramNode> for Emitter {
//     fn visit(&mut self, a: &QProgramNode) -> Result<()> {
//         // first loop: top level statements
//         for x in a.iter() {
//             match x {
//                 QTopLevelTokenNode::Statement(s) => self.visit(s)?,
//                 _ => ()
//             }
//         }

//         // add HALT instruction at end of program to separate from the functions and subs
//         // TODO: nice to have: use location of last statement
//         self
//         .instructions
//         .push(Instruction::Halt.at(Location::new(1, 1)));

//         // then functions and subs
//         for x in a.iter() {
//             match x {
//                 QTopLevelTokenNode::FunctionImplementation(n, params, block, pos) => {
//                     self.visit(&(n, params, block, pos))?;
//                 }
//                 QTopLevelTokenNode::SubImplementation(n, params, block, pos) => {
//                     self.visit(&(n, params, block, pos))?;
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// // function implementation
// impl Visitor<(&QNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)> for Emitter {
//     fn visit(&mut self, f: &(&QNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)) -> Result<()> {
//         let (name, params, block, pos) = *f;
//         let label = CaseInsensitiveString::new(format!(":fun:{}", name.bare_name()));
//         self.push(Instruction::Label(label), *pos);
//         // set default value
//         self.push(
//             Instruction::Load(Variant::default_variant(name.qualifier())), *pos
//         );
//         self
//             .push(Instruction::StoreAToResult, *pos);
//         self.visit(block)?;
//         self.push(Instruction::PopRet, *pos);
//         Ok(())
//     }
// }

// // sub implementation
// impl Visitor<(&BareNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)> for Emitter {
//     fn visit(&mut self, f: &(&BareNameNode, &Vec<QNameNode>, &QStatementNodes, &Location)) -> Result<()> {
//         let (name, params, block, pos) = *f;
//         let label = CaseInsensitiveString::new(format!(":sub:{}", name.bare_name()));
//         self.push(Instruction::Label(label), *pos);
//         self.visit(block)?;
//         self.push(Instruction::PopRet, *pos);
//         Ok(())
//     }
// }

// impl PostVisitor<Vec<QStatementNode>> for Emitter {
//     fn post_visit(&mut self, a: &Vec<QStatementNode>) ->Result<()> { Ok(()) }
// }
