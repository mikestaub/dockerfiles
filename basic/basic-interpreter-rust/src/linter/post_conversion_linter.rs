use super::error::*;
use super::types::*;
use crate::common::*;
use crate::parser::QualifiedName;

pub trait PostConversionLinter {
    fn visit_program(&self, p: &ProgramNode) -> Result<()> {
        p.iter()
            .map(|t| self.visit_top_level_token_node(t))
            .collect()
    }

    fn visit_top_level_token_node(&self, t: &TopLevelTokenNode) -> Result<()> {
        self.visit_top_level_token(t.as_ref())
            .map_err(|e| e.at_non_zero_location(t.location()))
    }

    fn visit_top_level_token(&self, t: &TopLevelToken) -> Result<()> {
        match t {
            TopLevelToken::FunctionImplementation(f) => self.visit_function_implementation(f),
            TopLevelToken::SubImplementation(s) => self.visit_sub_implementation(s),
            TopLevelToken::Statement(s) => self.visit_statement(s),
        }
    }

    fn visit_function_implementation(&self, f: &FunctionImplementation) -> Result<()> {
        self.visit_statement_nodes(&f.body)
    }

    fn visit_sub_implementation(&self, s: &SubImplementation) -> Result<()> {
        self.visit_statement_nodes(&s.body)
    }

    fn visit_statement_nodes(&self, s: &StatementNodes) -> Result<()> {
        s.iter().map(|x| self.visit_statement_node(x)).collect()
    }

    fn visit_statement_node(&self, t: &StatementNode) -> Result<()> {
        self.visit_statement(t.as_ref())
            .map_err(|e| e.at_non_zero_location(t.location()))
    }

    fn visit_statement(&self, s: &Statement) -> Result<()> {
        match s {
            Statement::SubCall(b, e) => self.visit_sub_call(b, e),
            Statement::ForLoop(f) => self.visit_for_loop(f),
            Statement::IfBlock(i) => self.visit_if_block(i),
            Statement::Assignment(left, right) => self.visit_assignment(left, right),
            Statement::While(w) => self.visit_conditional_block(w),
            Statement::Const(left, right) => self.visit_const(left, right),
            Statement::ErrorHandler(_label) => Ok(()),
            Statement::Label(_label) => Ok(()),
            Statement::GoTo(_label) => Ok(()),
            Statement::SetReturnValue(expr) => self.visit_expression(expr),
        }
    }

    fn visit_sub_call(
        &self,
        _name: &CaseInsensitiveString,
        args: &Vec<ExpressionNode>,
    ) -> Result<()> {
        args.iter().map(|e| self.visit_expression(e)).collect()
    }

    fn visit_assignment(&self, _name: &QualifiedName, v: &ExpressionNode) -> Result<()> {
        self.visit_expression(v)
    }

    fn visit_for_loop(&self, f: &ForLoopNode) -> Result<()> {
        self.visit_expression(&f.lower_bound)?;
        self.visit_expression(&f.upper_bound)?;
        match &f.step {
            Some(s) => self.visit_expression(s)?,
            None => (),
        }
        self.visit_statement_nodes(&f.statements)
    }

    fn visit_if_block(&self, i: &IfBlockNode) -> Result<()> {
        self.visit_conditional_block(&i.if_block)?;
        for else_if_block in i.else_if_blocks.iter() {
            self.visit_conditional_block(else_if_block)?;
        }
        match &i.else_block {
            Some(x) => self.visit_statement_nodes(x),
            None => Ok(()),
        }
    }

    fn visit_conditional_block(&self, c: &ConditionalBlockNode) -> Result<()> {
        self.visit_expression(&c.condition)?;
        self.visit_statement_nodes(&c.statements)
    }

    fn visit_const(&self, _left: &QNameNode, right: &ExpressionNode) -> Result<()> {
        self.visit_expression(right)
    }

    fn visit_expression(&self, _e: &ExpressionNode) -> Result<()> {
        Ok(())
    }
}
