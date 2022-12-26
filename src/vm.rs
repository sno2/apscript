use std::{collections::HashMap, fmt::Debug};

use gc::{Finalize, Gc, GcCell, Trace};
use rand::rngs::ThreadRng;

use crate::{
    ast::{BinaryOpKind, Expr, Node, Span, Stmt, UnaryOpKind},
    fail, tee,
};

#[derive(Trace, Finalize, Clone)]
pub enum Value {
    Void,
    Bool(bool),
    Number(f32),
    String(Gc<String>),
    Array(Gc<GcCell<Array>>),
    Builtin(Builtin),
    #[unsafe_ignore_trace]
    Exception(Box<Exception>),
}

#[derive(Debug, Finalize, Clone)]
pub struct Exception {
    pub message: String,
    pub span: Span,
}

unsafe impl Trace for Exception {
    unsafe fn trace(&self) {}

    unsafe fn root(&self) {}

    unsafe fn unroot(&self) {}

    fn finalize_glue(&self) {}
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Void => write!(f, "<void>"),
            Self::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) => {
                let mut iter = s.chars();
                while let Some(c) = iter.next() {
                    if c == '\\' {
                        iter.next();
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

#[derive(Debug, Finalize, Trace, Clone)]
pub struct Array {
    pub items: Vec<Value>,
}

pub struct VM<'a> {
    pub source: &'a str,
    pub scope: HashMap<&'a str, Value>,
    pub rng: Option<ThreadRng>,
}

impl<'a> VM<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            scope: HashMap::new(),
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
					fail!("expected index on array type", *span);
				};

                let Value::Number(idx) = tee!(self.eval_expr(index)) else {
					fail!("expected an integer index", *span);
				};

                let array = array.borrow();
                match array.items.get(idx as u32 as usize - 1) {
                    Some(v) => v.clone(),
                    None => fail!("array index is out of range", *span),
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
						fail!("expected a boolean type for operation", value.span());
					};

                    break 'blk Value::Bool(!b);
                }

                let Value::Number(n) = val else {
					fail!("expected a number type for operation", value.span());
				};
                Value::Number(if let UnaryOpKind::Pos = kind { n } else { -n })
            }
            Expr::BinaryOp { kind, lhs, rhs } => match kind {
                BinaryOpKind::And => 'blk: {
                    let Value::Bool(b1) = tee!(self.eval_expr(lhs)) else {
						fail!("expected a boolean for logical comparator", lhs.span());
					};

                    if !b1 {
                        break 'blk Value::Bool(false);
                    }

                    let Value::Bool(b2) = tee!(self.eval_expr(rhs)) else {
						fail!("expected a boolean for logical comparator", rhs.span());
					};

                    Value::Bool(b2)
                }
                BinaryOpKind::Or => 'blk: {
                    let Value::Bool(b1) = tee!(self.eval_expr(lhs)) else {
						fail!("expected a boolean for logical comparator", lhs.span());
					};

                    if b1 {
                        break 'blk Value::Bool(true);
                    }

                    let Value::Bool(b2) = tee!(self.eval_expr(rhs)) else {
						fail!("expected a boolean for logical comparator", rhs.span());
					};

                    Value::Bool(b2)
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
						fail!("expected a number type for operation", lhs.span());
					};

                    let Value::Number(n2) = rhs_value else {
						fail!("expected a number type for operation", rhs.span());
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
                _ => panic!(),
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
                        .insert(&self.source[Into::<std::ops::Range<_>>::into(*name)], v)
                        .finalize();
                }
                Stmt::Return { value, .. } => return self.eval_expr(value),
                Stmt::If {
                    cond,
                    scope,
                    else_ifs,
                    els,
                } => 'blk: {
                    let Value::Bool(b) = tee!(self.eval_expr(cond)) else {
						panic!();
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
                Stmt::For {
                    alias: _,
                    array,
                    scope,
                } => {
                    let arr = tee!(self.eval_expr(array));
                    let Value::Array(arr) = &arr else {
						fail!(format!("'{:?}' is not an array", array), array.span());
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