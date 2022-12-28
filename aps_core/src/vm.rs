use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    rc::Rc,
};

use gc::{Finalize, Gc, GcCell, Trace};
#[cfg(not(feature = "js"))]
use rand::rngs::ThreadRng;

use crate::{
    ast::{BinaryOpKind, Expr, Node, Procedure, Span, Stmt, UnaryOpKind},
    fail, tee,
};

#[derive(Trace, Finalize, Clone)]
pub enum Value {
    Void,
    Bool(bool),
    Number(f32),
    String(Gc<String>),
    Array(Gc<GcCell<Array>>),
    #[unsafe_ignore_trace]
    Builtin(Builtin),
    #[unsafe_ignore_trace]
    Procedure(Rc<Procedure>),
    #[unsafe_ignore_trace]
    Exception(Box<Exception>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Array(l0), Self::Array(r0)) => l0 == r0,
            (Self::Builtin(l0), Self::Builtin(r0)) => l0.0 as usize == r0.0 as usize,
            (Self::Exception(_), Self::Exception(_)) => false,
            _ => false,
        }
    }
}

#[derive(Debug, Finalize, Clone)]
pub struct Exception {
    pub message: String,
    pub span: Span,
    pub stack: Vec<Span>,
}

unsafe impl Trace for Exception {
    unsafe fn trace(&self) {}

    unsafe fn root(&self) {}

    unsafe fn unroot(&self) {}

    fn finalize_glue(&self) {}
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "<void>"),
            Self::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Self::Number(n) => write!(f, "{}", n),
            Self::Procedure(_) => write!(f, "<procedure>"),
            Self::String(s) => {
                let mut iter = s.chars();
                while let Some(c) = iter.next() {
                    if c == '\\' {
                        iter.next();
                        continue;
                    }
                    write!(f, "{c}")?;
                }
                Ok(())
            }
            Self::Exception(_) => unreachable!(),
            Self::Array(array) => {
                write!(f, "[")?;
                let array = &array.borrow().items;
                let mut iter = array.iter();

                if let Some(v0) = iter.next() {
                    write!(f, "{:?}", v0)?;
                    for itm in iter {
                        write!(f, ", {:?}", itm)?;
                    }
                }
                write!(f, "]")
            }
            Self::Builtin(_) => write!(f, "<builtin>"),
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{:?}", s),
            _ => write!(f, "{}", self),
        }
    }
}

#[derive(Finalize, Clone, Copy)]
pub struct Builtin(pub BuiltinPtr);

unsafe impl Trace for Builtin {
    unsafe fn trace(&self) {}

    unsafe fn root(&self) {}

    unsafe fn unroot(&self) {}

    fn finalize_glue(&self) {}
}

pub type BuiltinPtr = fn(&mut VM, &[Value]) -> Value;

impl Debug for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builtin").finish_non_exhaustive()
    }
}

#[derive(PartialEq, Debug, Finalize, Trace, Clone)]
pub struct Array {
    pub items: Vec<Value>,
}

pub struct VM<'a> {
    pub source: &'a str,
    pub scope: HashMap<&'a str, Value>,

    #[cfg(not(feature = "js"))]
    pub rng: Option<ThreadRng>,
}

impl<'a> VM<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            scope: HashMap::new(),
            #[cfg(not(feature = "js"))]
            rng: None,
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::Void => unreachable!(),
            Expr::BinaryLiteral { .. } | Expr::HexLiteral { .. } => panic!(),
            Expr::Index { value, index, span } => {
                let v = tee!(self.eval_expr(value));

                let Value::Array(array) = &v else {
					fail!(format!("{v:?} is not an array"), *span);
				};

                let idx = tee!(self.eval_expr(index));
                let Value::Number(idx) = idx else {
					fail!(format!("{idx:?} is not an integer"), *span);
				};

                let array = array.borrow();
                match array.items.get(idx as u32 as usize - 1) {
                    Some(v) => v.clone(),
                    None => fail!(
                        format!(
                            "index {idx} is out of array range (length: {})",
                            array.items.len()
                        ),
                        *span
                    ),
                }
            }
            Expr::True { .. } => Value::Bool(true),
            Expr::False { .. } => Value::Bool(false),
            Expr::IntegerLiteral { span } | Expr::FloatLiteral { span } => Value::Number(
                self.source[Into::<std::ops::Range<_>>::into(*span)]
                    .parse()
                    .unwrap(),
            ),
            &Expr::Identifier { span } => {
                let name = &self.source[Into::<std::ops::Range<_>>::into(span)];
                let Some(value) = self.scope.get(name) else {
					fail!(format!("'{}' is not defined", name), span);
				};
                value.clone()
            }
            &Expr::StringLiteral { span } => Value::String(Gc::new(String::from(
                &self.source[Into::<std::ops::Range<_>>::into(Span {
                    start: span.start + 1,
                    end: span.end - 1,
                })],
            ))),
            Expr::UnaryOp { kind, value, .. } => 'blk: {
                let val = tee!(self.eval_expr(value));

                if let UnaryOpKind::Not = kind {
                    let Value::Bool(b) = val else {
						fail!(format!("{val:?} is not a boolean"), value.span());
					};

                    break 'blk Value::Bool(!b);
                }

                let Value::Number(n) = val else {
					fail!(format!("{val:?} is not a boolean"), value.span());
				};
                Value::Number(if let UnaryOpKind::Pos = kind { n } else { -n })
            }
            Expr::BinaryOp { kind, lhs, rhs } => match kind {
                BinaryOpKind::And => 'blk: {
                    let lhs_value = tee!(self.eval_expr(lhs));
                    let Value::Bool(b1) = lhs_value else {
						fail!(format!("{lhs_value:?} is not a boolean"), lhs.span());
					};

                    if !b1 {
                        break 'blk Value::Bool(false);
                    }

                    let rhs_value = tee!(self.eval_expr(rhs));
                    let Value::Bool(b2) = rhs_value else {
						fail!(format!("{rhs_value:?} is not a boolean"), rhs.span());
					};

                    Value::Bool(b2)
                }
                BinaryOpKind::Or => 'blk: {
                    let lhs_value = tee!(self.eval_expr(lhs));
                    let Value::Bool(b1) = lhs_value else {
						fail!(format!("{lhs_value:?} is not a boolean"), lhs.span());
					};

                    if b1 {
                        break 'blk Value::Bool(true);
                    }

                    let rhs_value = tee!(self.eval_expr(rhs));
                    let Value::Bool(b2) = rhs_value else {
						fail!(format!("{rhs_value:?} is not a boolean"), rhs.span());
					};

                    Value::Bool(b2)
                }
                BinaryOpKind::Equal => {
                    let lhs_value = tee!(self.eval_expr(lhs));
                    let rhs_value = tee!(self.eval_expr(rhs));

                    Value::Bool(lhs_value == rhs_value)
                }
                BinaryOpKind::NotEqual => {
                    let lhs_value = tee!(self.eval_expr(lhs));
                    let rhs_value = tee!(self.eval_expr(rhs));

                    Value::Bool(lhs_value != rhs_value)
                }
                BinaryOpKind::Add
                | BinaryOpKind::Sub
                | BinaryOpKind::Mul
                | BinaryOpKind::Div
                | BinaryOpKind::Mod
                | BinaryOpKind::Greater
                | BinaryOpKind::GreaterEqual
                | BinaryOpKind::Less
                | BinaryOpKind::LessEqual => {
                    let lhs_value = tee!(self.eval_expr(lhs));
                    let rhs_value = tee!(self.eval_expr(rhs));

                    let Value::Number(n1) = lhs_value else {
						fail!(format!("{lhs_value:?} is not a number"), lhs.span());
					};

                    let Value::Number(n2) = rhs_value else {
						fail!(format!("{rhs_value:?} is not a number"), rhs.span());
					};

                    match kind {
                        BinaryOpKind::Add => Value::Number(n1 + n2),
                        BinaryOpKind::Sub => Value::Number(n1 - n2),
                        BinaryOpKind::Mul => Value::Number(n1 * n2),
                        BinaryOpKind::Div => Value::Number(n1 / n2),
                        BinaryOpKind::Mod => Value::Number(n1 % n2),
                        BinaryOpKind::Greater => Value::Bool(n1 > n2),
                        BinaryOpKind::GreaterEqual => Value::Bool(n1 >= n2),
                        BinaryOpKind::Less => Value::Bool(n1 < n2),
                        BinaryOpKind::LessEqual => Value::Bool(n1 <= n2),
                        _ => unreachable!(),
                    }
                }
            },
            Expr::Paren { value, .. } => tee!(self.eval_expr(value)),
            Expr::ArrayLiteral { values, .. } => {
                let mut items = Vec::with_capacity(values.len());

                for v in values.iter() {
                    items.push(tee!(self.eval_expr(v)));
                }

                Value::Array(Gc::new(GcCell::new(Array { items })))
            }
            Expr::FnCall { calle, args, span } => 'blk: {
                let v = tee!(self.eval_expr(calle));

                if let Value::Procedure(proc) = &v {
                    let res = self.eval_scope(&proc.scope);

                    if let Value::Exception(e) = &res {
                        let mut e = e.clone();
                        e.stack.push(*span);
                        break 'blk Value::Exception(e);
                    };

                    break 'blk res;
                }

                if let Value::Builtin(calle) = &v {
                    let mut oargs = Vec::with_capacity(args.len());

                    for arg in args.iter() {
                        oargs.push(tee!(self.eval_expr(arg)));
                    }

                    let res = calle.0(self, &oargs);

                    break 'blk if let Value::Exception(e) = &res {
                        Value::Exception(Box::new(Exception {
                            message: e.message.clone(),
                            span: *span,
                            stack: Vec::new(),
                        }))
                    } else {
                        res
                    };
                }

                fail!(format!("{v:?} is not a function"), calle.span());
            }
        }
    }

    pub fn eval_scope(&mut self, scope: &[Stmt]) -> Value {
        for stmt in scope.iter() {
            match stmt {
                Stmt::Expr(e) => _ = tee!(self.eval_expr(e)),
                Stmt::VarAssign { name, value } => {
                    let v = tee!(self.eval_expr(value));
                    self.scope
                        .insert(&self.source[Into::<std::ops::Range<_>>::into(*name)], v);
                }
                Stmt::Procedure(proc) => {
                    // TODO: this clone is wildly inefficient
                    self.scope.insert(
                        &self.source[Into::<std::ops::Range<_>>::into(proc.name)],
                        Value::Procedure(Rc::new(proc.clone())),
                    );
                }
                Stmt::IndexAssign { root, index, value } => {
                    let rootv = tee!(self.eval_expr(root));
                    let Value::Array(rootv) = &rootv else {
						fail!(format!("{rootv:?} is not an array"), root.span());
					};

                    let indexv = tee!(self.eval_expr(index));
                    let Value::Number(idx) = &indexv else {
						fail!(format!("{indexv:?} is not a number"), index.span());
					};

                    let mut rootv = rootv.borrow_mut();
                    let Some(vptr) = rootv.items.get_mut(*idx as u32 as usize - 1) else {
						fail!(format!("index is out of bounds: the length is {:?} but the index is {idx}", rootv.items.len()), stmt.span());
					};

                    *vptr = tee!(self.eval_expr(value));
                }
                Stmt::Return { value, .. } => return self.eval_expr(value),
                Stmt::If {
                    cond,
                    scope,
                    else_ifs,
                    els,
                } => 'blk: {
                    let c1 = tee!(self.eval_expr(cond));
                    let Value::Bool(b) = c1 else {
						fail!(format!("{c1:?} is not a boolean"), cond.span());
					};

                    if b {
                        let scope_val = tee!(self.eval_scope(scope));

                        let Value::Void = scope_val else {
                            return scope_val;
                        };

                        break 'blk;
                    }

                    for else_if in else_ifs.iter() {
                        let Value::Bool(b) = tee!(self.eval_expr(&else_if.cond)) else {
							panic!();
						};

                        if b {
                            let scope_val = tee!(self.eval_scope(&else_if.scope));

                            let Value::Void = scope_val else {
								return scope_val;
							};

                            break 'blk;
                        }
                    }

                    if let Some(els) = els {
                        let scope_val = tee!(self.eval_scope(els));

                        let Value::Void = scope_val else {
							return scope_val;
						};

                        break 'blk;
                    }
                }
                Stmt::RepeatN { n: n_expr, scope } => {
                    let count = tee!(self.eval_expr(n_expr));

                    let Value::Number(n) = count else {
						fail!(format!("{count:?} is not a number"), n_expr.span());
					};

                    if n < 0. {
                        fail!(format!("{count:?} is not positive"), n_expr.span());
                    }

                    if n.floor() != n {
                        fail!(format!("{count:?} is not an integer"), n_expr.span());
                    }

                    let mut n = n as u32;

                    while n > 0 {
                        let val = tee!(self.eval_scope(scope));

                        let Value::Void = val else {
							return val;
						};

                        n -= 1;
                    }
                }
                Stmt::RepeatUntil { cond, scope } => loop {
                    let val = tee!(self.eval_expr(cond));

                    let Value::Bool(b) = val else {
						fail!(format!("{val:?} is not a boolean"), cond.span());
					};

                    if b {
                        break;
                    }

                    let val = tee!(self.eval_scope(scope));
                    let Value::Void = val else {
						return val;
					};
                },
                Stmt::For {
                    alias: _,
                    array,
                    scope,
                } => {
                    let arr = tee!(self.eval_expr(array));
                    let Value::Array(arr) = &arr else {
						fail!(format!("{:?} is not an array", array), array.span());
					};

                    let mut i = 0;
                    let len = arr.borrow().items.len();

                    loop {
                        if i >= len {
                            break;
                        }

                        let scope_val = tee!(self.eval_scope(scope));

                        let Value::Void = scope_val else {
							return scope_val;
						};

                        i += 1;
                    }
                }
            }
        }
        Value::Void
    }
}