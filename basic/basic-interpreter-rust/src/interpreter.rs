mod assignment;
mod built_in_functions;
mod built_in_subs;
mod casting;
mod constant;
mod context;
mod context_owner;
mod emitter;
mod expression;
mod for_loop;
mod function_call;
mod function_context;
mod go_to;
mod if_block;
mod instruction;
mod interpreter_error;
mod statement;
mod stdlib;
mod sub_call;
mod sub_context;
mod subprogram_context;
mod subprogram_resolver;
mod variant;
mod while_wend;

#[cfg(test)]
mod test_utils;

pub use self::emitter::*;
pub use self::instruction::*;
pub use self::interpreter_error::*;
pub use self::stdlib::*;
pub use self::variant::*;

use crate::common::*;
use crate::interpreter::casting::cast;
use crate::interpreter::context::Context;
use crate::interpreter::context_owner::ContextOwner;
use crate::interpreter::function_context::{FunctionContext, QualifiedFunctionImplementationNode};
use crate::interpreter::sub_context::{QualifiedSubImplementationNode, SubContext};
use crate::parser::type_resolver_impl::TypeResolverImpl;
use crate::parser::*;

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::rc::Rc;

// TODO: 1. instructionContext -> emitter
//       2. fix bug
//       3. hashmap<?,Hashmap<?>> -> new class / classes in context
//       4. context enums
//       5. fix remaining todos
//       6. rename suprogram_resolver to linter

impl<T: TypeResolver> TypeResolver for Rc<RefCell<T>> {
    fn resolve<U: NameTrait>(&self, name: &U) -> TypeQualifier {
        self.as_ref().borrow().resolve(name)
    }
}

#[derive(Debug)]
pub struct Registers(Variant, Variant);

pub type RegisterStack = VecDeque<Registers>;

#[derive(Debug)]
pub struct Interpreter<S: Stdlib> {
    stdlib: S,
    context: Option<Context<TypeResolverImpl>>,
    function_context: FunctionContext,
    sub_context: SubContext,
    type_resolver: Rc<RefCell<TypeResolverImpl>>,
    register_stack: RegisterStack,
    return_stack: Vec<usize>,
}

pub type Result<T> = std::result::Result<T, InterpreterError>;

impl<TStdlib: Stdlib> Interpreter<TStdlib> {
    pub fn new(stdlib: TStdlib) -> Self {
        let tr = Rc::new(RefCell::new(TypeResolverImpl::new()));
        let mut result = Interpreter {
            stdlib,
            context: Some(Context::new(Rc::clone(&tr))),
            function_context: FunctionContext::new(),
            sub_context: SubContext::new(),
            type_resolver: tr,
            return_stack: vec![],
            register_stack: VecDeque::new(),
        };
        result
            .register_stack
            .push_back(Registers(Variant::VInteger(0), Variant::VInteger(0)));
        result
    }

    fn generate_instructions_unresolved(
        &mut self,
        program: ProgramNode,
    ) -> Result<Vec<InstructionNode>> {
        let mut results = InstructionContext::new();
        for x in program {
            match x {
                TopLevelTokenNode::Statement(s) => {
                    self.generate_statement_instructions(&mut results, s)?;
                }
                TopLevelTokenNode::DefType(d, pos) => {
                    results.push(Instruction::DefType(d), pos);
                }
                _ => unimplemented!(),
            }
        }

        // add HALT instruction at end of program to separate from the functions and subs
        // TODO: nice to have: use location of last statement
        results.push(Instruction::Halt, Location::start());

        // functions
        for x in self.function_context.implementations.clone().into_iter() {
            let (k, v) = x;
            let pos = v.location();
            let name = v.name;
            let params = v.parameters;
            let block = v.block;
            let label = CaseInsensitiveString::new(format!(":fun:{}", name.bare_name()));
            results.push(Instruction::Label(label), pos);
            // set default value
            results.push(
                Instruction::Load(Variant::default_variant(name.qualifier())),
                pos,
            );
            results.push(Instruction::StoreAToResult, pos);
            self.generate_block_instructions(&mut results, block)?;
            results.push(Instruction::PopRet, pos);
        }

        // subs
        for x in self.sub_context.implementations.clone().into_iter() {
            let (k, v) = x;
            let pos = v.location();
            let name = v.name;
            let params = v.parameters;
            let block = v.block;
            let label = CaseInsensitiveString::new(format!(":sub:{}", name.bare_name()));
            results.push(Instruction::Label(label), pos);
            self.generate_block_instructions(&mut results, block)?;
            results.push(Instruction::PopRet, pos);
        }

        Ok(results.instructions)
    }

    pub fn generate_instructions(&mut self, program: ProgramNode) -> Result<Vec<InstructionNode>> {
        let mut instruction_nodes = self.generate_instructions_unresolved(program)?;
        let labels = Self::collect_labels(&instruction_nodes);
        // resolve jumps
        for instruction_node in instruction_nodes.iter_mut() {
            let instruction: &Instruction = instruction_node.as_ref();
            let pos: Location = instruction_node.location();
            if let Instruction::UnresolvedJump(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::Jump(*idx).at(pos);
                    }
                    None => {
                        return Err(InterpreterError::new_with_pos("Label not found", pos));
                    }
                }
            } else if let Instruction::UnresolvedJumpIfFalse(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::JumpIfFalse(*idx).at(pos);
                    }
                    None => {
                        return Err(InterpreterError::new_with_pos("Label not found", pos));
                    }
                }
            } else if let Instruction::SetUnresolvedErrorHandler(x) = instruction {
                match labels.get(x) {
                    Some(idx) => {
                        *instruction_node = Instruction::SetErrorHandler(*idx).at(pos);
                    }
                    None => {
                        return Err(InterpreterError::new_with_pos("Label not found", pos));
                    }
                }
            }
        }
        Ok(instruction_nodes)
    }

    fn collect_labels(
        instructions: &Vec<InstructionNode>,
    ) -> HashMap<CaseInsensitiveString, usize> {
        let mut result: HashMap<CaseInsensitiveString, usize> = HashMap::new();
        for j in 0..instructions.len() {
            if let Instruction::Label(y) = instructions[j].as_ref() {
                result.insert(y.clone(), j);
            }
        }
        result
    }

    fn sanitize(&mut self, original_program: ProgramNode) -> Result<ProgramNode> {
        subprogram_resolver::NoFunctionInConst::no_function_in_const(&original_program)?;
        subprogram_resolver::for_next_counter_match(&original_program)?;
        let (program, f_c, s_c) = subprogram_resolver::resolve(original_program)?;
        subprogram_resolver::AllSubsKnown::all_subs_known(&program, &s_c)?;
        subprogram_resolver::AllFunctionsKnown::all_functions_known(&program, &f_c)?;
        self.function_context = f_c;
        self.sub_context = s_c;
        Ok(program)
    }

    fn get_a(&self) -> Variant {
        self.register_stack.back().unwrap().0.clone()
    }

    fn get_b(&self) -> &Variant {
        &self.register_stack.back().unwrap().1
    }

    fn set_a(&mut self, v: Variant) {
        self.register_stack.back_mut().unwrap().0 = v;
    }

    fn set_b(&mut self, v: Variant) {
        self.register_stack.back_mut().unwrap().1 = v;
    }

    fn interpret_one(
        &mut self,
        i: &mut usize,
        instruction: &Instruction,
        pos: Location,
        error_handler: &mut Option<usize>,
        exit: &mut bool,
    ) -> Result<()> {
        match instruction {
            Instruction::SetErrorHandler(idx) => {
                *error_handler = Some(*idx);
            }
            Instruction::PushRegisters => {
                self.register_stack
                    .push_back(Registers(Variant::VInteger(0), Variant::VInteger(0)));
            }
            Instruction::PopRegisters => {
                let old_registers = self.register_stack.pop_back();
                self.set_a(old_registers.unwrap().0);
            }
            Instruction::Load(v) => {
                self.set_a(v.clone());
            }
            Instruction::Store(n) => {
                let v = self.get_a();
                self.context_mut().set_l_value(n, pos, v)?;
            }
            Instruction::StoreConst(n) => {
                let v = self.get_a();
                self.context_mut()
                    .set_const_l_value(&Name::Bare(n.clone()).at(pos), v)?;
            }
            Instruction::Cast(q) => {
                let v = cast(self.get_a(), *q)
                    .map_err(|msg| InterpreterError::new_with_pos(msg, pos))?;
                self.set_a(v);
            }
            Instruction::CopyAToB => {
                let v = self.get_a();
                self.set_b(v);
            }
            Instruction::Plus => {
                let a = self.get_a();
                let b = self.get_b();
                self.set_a(
                    a.plus(&b)
                        .map_err(|e| InterpreterError::new_with_pos(e, pos))?,
                );
            }
            Instruction::Minus => {
                let a = self.get_a();
                let b = self.get_b();
                self.set_a(
                    a.minus(&b)
                        .map_err(|e| InterpreterError::new_with_pos(e, pos))?,
                );
            }
            Instruction::NegateA => {
                let a = self.get_a();
                self.set_a(
                    a.negate()
                        .map_err(|e| InterpreterError::new_with_pos(e, pos))?,
                );
            }
            Instruction::NotA => {
                let a = self.get_a();
                self.set_a(
                    a.unary_not()
                        .map_err(|e| InterpreterError::new_with_pos(e, pos))?,
                );
            }
            Instruction::CopyVarToA(n) => {
                let name_node: NameNode = n.clone().at(pos);
                match self.context_ref().get_r_value(&name_node)? {
                    Some(v) => self.set_a(v),
                    None => panic!("Variable {} undefined at {:?}", n, pos),
                }
            }
            Instruction::CopyVarToB(n) => {
                let name_node: NameNode = n.clone().at(pos);
                let v = self.context_ref().get_r_value(&name_node)?.unwrap().clone();
                self.set_b(v);
            }
            Instruction::LessThan => {
                let a = self.get_a();
                let b = self.get_b();
                let order = a
                    .cmp(&b)
                    .map_err(|e| InterpreterError::new_with_pos(e, pos))?;
                let is_true = order == Ordering::Less;
                self.set_a(is_true.into());
            }
            Instruction::GreaterThan => {
                let a = self.get_a();
                let b = self.get_b();
                let order = a
                    .cmp(&b)
                    .map_err(|e| InterpreterError::new_with_pos(e, pos))?;
                let is_true = order == Ordering::Greater;
                self.set_a(is_true.into());
            }
            Instruction::LessOrEqualThan => {
                let a = self.get_a();
                let b = self.get_b();
                let order = a
                    .cmp(&b)
                    .map_err(|e| InterpreterError::new_with_pos(e, pos))?;
                let is_true = order == Ordering::Less || order == Ordering::Equal;
                self.set_a(is_true.into());
            }
            Instruction::GreaterOrEqualThan => {
                let a = self.get_a();
                let b = self.get_b();
                let order = a
                    .cmp(&b)
                    .map_err(|e| InterpreterError::new_with_pos(e, pos))?;
                let is_true = order == Ordering::Greater || order == Ordering::Equal;
                self.set_a(is_true.into());
            }
            Instruction::JumpIfFalse(resolved_idx) => {
                let a = self.get_a();
                let is_true: bool = (&a)
                    .try_into()
                    .map_err(|e| InterpreterError::new_with_pos(e, pos))?;
                if !is_true {
                    *i = resolved_idx - 1; // the +1 will happen at the end of the loop
                }
            }
            Instruction::Jump(resolved_idx) => {
                *i = resolved_idx - 1;
            }
            Instruction::PreparePush => {
                self.push_args_context();
            }
            Instruction::PushStack => {
                self.swap_args_with_sub_context();
            }
            Instruction::PopStack => {
                self.pop();
            }
            Instruction::PushUnnamedRefParam(name) => {
                self.context_mut()
                    .demand_args()
                    .push_back_unnamed_ref_parameter(&name.clone().at(pos))?;
            }
            Instruction::PushUnnamedValParam => {
                let v = self.get_a();

                self.context_mut()
                    .demand_args()
                    .push_back_unnamed_val_parameter(v);
            }
            Instruction::SetNamedRefParam(param_q_name, ref_name) => {
                self.context_mut()
                    .demand_args()
                    .set_named_ref_parameter(param_q_name, &ref_name.clone().at(pos))?;
            }
            Instruction::SetNamedValParam(param_q_name) => {
                let v = self.get_a();

                self.context_mut()
                    .demand_args()
                    .set_named_val_parameter(param_q_name, v);
            }
            Instruction::BuiltInSub(n) => {
                self.run_built_in_sub(n, pos)?;
            }
            Instruction::BuiltInFunction(n) => {
                self.run_built_in_function(n, pos)?;
            }
            Instruction::UnresolvedJump(_) | Instruction::UnresolvedJumpIfFalse(_) => {
                panic!("Unresolved label {:?} at {:?}", instruction, pos)
            }
            Instruction::Label(_) => (), // no-op
            Instruction::DefType(def_type) => {
                self.handle_def_type(def_type);
            }
            Instruction::Halt => {
                *exit = true;
            }
            Instruction::PushRet(addr) => {
                self.return_stack.push(*addr);
            }
            Instruction::PopRet => {
                let addr = self.return_stack.pop().unwrap();
                *i = addr - 1;
            }
            Instruction::StoreAToResult => {
                let v = self.get_a();
                self.context_mut().demand_sub().set_function_result(v);
            }
            Instruction::CopyResultToA => {
                let v = self.context_ref().get_function_result().clone();
                self.set_a(v);
            }
            Instruction::Throw(msg) => {
                self.throw(msg, pos)?;
            }
            _ => unimplemented!("{:?}", instruction),
        }
        Ok(())
    }

    pub fn interpret(&mut self, original_program: ProgramNode) -> Result<()> {
        let program = self.sanitize(original_program)?;

        let instructions = self.generate_instructions(program)?;
        let mut i: usize = 0;
        let mut error_handler: Option<usize> = None;
        let mut exit: bool = false;
        while i < instructions.len() && !exit {
            let instruction = instructions[i].as_ref();
            let pos = instructions[i].location();
            match self.interpret_one(&mut i, instruction, pos, &mut error_handler, &mut exit) {
                Ok(_) => {
                    i += 1;
                }
                Err(e) => match error_handler {
                    Some(error_idx) => {
                        i = error_idx;
                    }
                    None => {
                        return Err(e);
                    }
                },
            }
        }
        Ok(())
    }

    fn throw(&mut self, msg: &String, pos: Location) -> Result<()> {
        Err(InterpreterError::new_with_pos(msg, pos))
    }

    fn handle_def_type(&mut self, x: &DefType) {
        self.type_resolver.borrow_mut().set(x);
    }
}

pub trait LookupFunctionImplementation {
    fn has_function(&self, function_name: &NameNode) -> bool;

    fn lookup_function_implementation(
        &self,
        function_name: &NameNode,
    ) -> Option<QualifiedFunctionImplementationNode>;
}

impl<S: Stdlib> LookupFunctionImplementation for Interpreter<S> {
    fn has_function(&self, function_name: &NameNode) -> bool {
        self.function_context
            .has_implementation(function_name.bare_name())
    }

    fn lookup_function_implementation(
        &self,
        function_name: &NameNode,
    ) -> Option<QualifiedFunctionImplementationNode> {
        self.function_context.get_implementation(function_name)
    }
}

pub trait LookupSubImplementation {
    fn has_sub(&self, sub_name: &BareNameNode) -> bool;

    fn get_sub(&self, sub_name: &BareNameNode) -> QualifiedSubImplementationNode;
}

impl<S: Stdlib> LookupSubImplementation for Interpreter<S> {
    fn has_sub(&self, sub_name: &BareNameNode) -> bool {
        self.sub_context.has_implementation(sub_name.as_ref())
    }

    fn get_sub(&self, sub_name: &BareNameNode) -> QualifiedSubImplementationNode {
        self.sub_context
            .get_implementation(sub_name.as_ref())
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;

    #[test]
    fn test_interpret_print_hello_world() {
        let input = "PRINT \"Hello, world!\"";
        assert_eq!(interpret(input).stdlib.output, vec!["Hello, world!"]);
    }

    #[test]
    fn test_interpret_print_hello_world_two_args() {
        let input = r#"PRINT "Hello", "world!""#;
        assert_eq!(interpret(input).stdlib.output, vec!["Hello world!"]);
    }

    #[test]
    fn test_interpret_print_hello_world_two_args_one_is_function() {
        let input = r#"
        PRINT "Hello", Test(1)
        FUNCTION Test(N)
            Test = N + 1
        END FUNCTION
        "#;
        assert_eq!(interpret(input).stdlib.output, vec!["Hello 2"]);
    }

    #[test]
    fn test_interpreter_fixture_hello1() {
        let stdlib = MockStdlib::new();
        interpret_file("HELLO1.BAS", stdlib).unwrap();
    }

    #[test]
    fn test_interpreter_fixture_hello2() {
        let stdlib = MockStdlib::new();
        interpret_file("HELLO2.BAS", stdlib).unwrap();
    }

    #[test]
    fn test_interpreter_fixture_hello_s() {
        let stdlib = MockStdlib::new();
        interpret_file("HELLO_S.BAS", stdlib).unwrap();
    }

    #[test]
    fn test_interpreter_for_print_10() {
        let stdlib = MockStdlib::new();
        interpret_file("FOR_PRINT_10.BAS", stdlib).unwrap();
    }

    #[test]
    fn test_interpreter_for_nested() {
        let stdlib = MockStdlib::new();
        interpret_file("FOR_NESTED.BAS", stdlib).unwrap();
    }

    #[test]
    fn test_interpreter_fixture_fib_bas() {
        let mut stdlib = MockStdlib::new();
        stdlib.add_next_input("10");
        let interpreter = interpret_file("FIB.BAS", stdlib).unwrap();
        let output = interpreter.stdlib.output;
        assert_eq!(
            output,
            vec![
                "Enter the number of fibonacci to calculate",
                "Fibonacci of 0 is 0",
                "Fibonacci of 1 is 1",
                "Fibonacci of 2 is 1",
                "Fibonacci of 3 is 2",
                "Fibonacci of 4 is 3",
                "Fibonacci of 5 is 5",
                "Fibonacci of 6 is 8",
                "Fibonacci of 7 is 13",
                "Fibonacci of 8 is 21",
                "Fibonacci of 9 is 34",
                "Fibonacci of 10 is 55"
            ]
        );
    }

    #[test]
    fn test_interpreter_fixture_fib_fq_bas() {
        let mut stdlib = MockStdlib::new();
        stdlib.add_next_input("11");
        interpret_file("FIB_FQ.BAS", stdlib).unwrap();
    }

    #[test]
    fn test_interpreter_fixture_input() {
        let mut stdlib = MockStdlib::new();
        stdlib.add_next_input("");
        interpret_file("INPUT.BAS", stdlib).unwrap();
    }
}
