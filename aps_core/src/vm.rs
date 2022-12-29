use std::{
    cell::RefCell,
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

    #[cfg(not(feature = "js"))]
    pub rng: Option<ThreadRng>,
}

// Inspired by burdonsmith's rust_lisp implementation
pub struct Env<'a> {
    pub parent: Option<Rc<RefCell<Env<'a>>>>,
    pub entries: HashMap<String, Value>,
}

impl Env<'_> {
    pub fn new() -> Self {
        Self {
            parent: None,
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, s: &str) -> Option<Value> {
        if let Some(val) = self.entries.get(s) {
            Some(val.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get(s)
        } else {
            None
        }
    }
}

impl<'a> VM<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            #[cfg(not(feature = "js"))]
            rng: None,
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr, env: Rc<RefCell<Env>>) -> Value {
        match expr {
            Expr::Void => unreachable!(),
            Expr::BinaryLiteral { .. } | Expr::HexLiteral { .. } => panic!(),
            Expr::Index { value, index, span } => {
                let v = tee!(self.eval_expr(value, env.clone()));

                let Value::Array(array) = &v else {
					fail!(format!("{v:?} is not an array"), *span);
				};

                let idx = tee!(self.eval_expr(index, env));
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

                let Some(v) = env.borrow().get(name) else {
					fail!(format!("'{}' is not defined", name), span);
				};

                v
            }
            &Expr::StringLiteral { span } => Value::String(Gc::new(String::from(
                &self.source[Into::<std::ops::Range<_>>::into(Span {
                    start: span.start + 1,
                    end: span.end - 1,
                })],
            ))),
            Expr::UnaryOp { kind, value, .. } => 'blk: {
                let val = tee!(self.eval_expr(value, env));

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
                    let lhs_value = tee!(self.eval_expr(lhs, env.clone()));
                    let Value::Bool(b1) = lhs_value else {
						fail!(format!("{lhs_value:?} is not a boolean"), lhs.span());
					};

                    if !b1 {
                        break 'blk Value::Bool(false);
                    }

                    let rhs_value = tee!(self.eval_expr(rhs, env));
                    let Value::Bool(b2) = rhs_value else {
						fail!(format!("{rhs_value:?} is not a boolean"), rhs.span());
					};

                    Value::Bool(b2)
                }
                BinaryOpKind::Or => 'blk: {
                    let lhs_value = tee!(self.eval_expr(lhs, env.clone()));
                    let Value::Bool(b1) = lhs_value else {
						fail!(format!("{lhs_value:?} is not a boolean"), lhs.span());
					};

                    if b1 {
                        break 'blk Value::Bool(true);
                    }

                    let rhs_value = tee!(self.eval_expr(rhs, env));
                    let Value::Bool(b2) = rhs_value else {
						fail!(format!("{rhs_value:?} is not a boolean"), rhs.span());
					};

                    Value::Bool(b2)
                }
                BinaryOpKind::Equal => {
                    let lhs_value = tee!(self.eval_expr(lhs, env.clone()));
                    let rhs_value = tee!(self.eval_expr(rhs, env));

                    Value::Bool(lhs_value == rhs_value)
                }
                BinaryOpKind::NotEqual => {
                    let lhs_value = tee!(self.eval_expr(lhs, env.clone()));
                    let rhs_value = tee!(self.eval_expr(rhs, env));

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
                    let lhs_value = tee!(self.eval_expr(lhs, env.clone()));
                    let rhs_value = tee!(self.eval_expr(rhs, env));

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
            Expr::Paren { value, .. } => tee!(self.eval_expr(value, env)),
            Expr::ArrayLiteral { values, .. } => {
                let mut items = Vec::with_capacity(values.len());

                for v in values.iter() {
                    items.push(tee!(self.eval_expr(v, env.clone())));
                }

                Value::Array(Gc::new(GcCell::new(Array { items })))
            }
            Expr::FnCall { calle, args, span } => 'blk: {
                let v = tee!(self.eval_expr(calle, env.clone()));

                if let Value::Procedure(proc) = &v {
                    if args.len() != proc.params.len() {
                        fail!(
                            format!(
                                "expected {} arguments, found {}",
                                proc.params.len(),
                                args.len()
                            ),
                            *span
                        );
                    }

                    let mut child_env = Env {
                        parent: Some(env.clone()),
                        entries: HashMap::new(),
                    };

                    for (idx, arg) in args.iter().enumerate() {
                        let argv = tee!(self.eval_expr(arg, env.clone()));
                        child_env.entries.insert(
                            self.source[Into::<std::ops::Range<_>>::into(proc.params[idx])].into(),
                            argv,
                        );
                    }

                    let res = self.eval_scope(&proc.scope, Rc::new(RefCell::new(child_env)));

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
                        oargs.push(tee!(self.eval_expr(arg, env.clone())));
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

    pub fn eval_scope(&mut self, scope: &[Stmt], env: Rc<RefCell<Env>>) -> Value {
        for stmt in scope.iter() {
            match stmt {
                Stmt::Expr(e) => _ = tee!(self.eval_expr(e, env.clone())),
                Stmt::VarAssign { name, value } => {
                    let v = tee!(self.eval_expr(value, env.clone())).clone();
                    let mut cur_env = env.clone();
                    let name = self.source[Into::<std::ops::Range<_>>::into(*name)].to_string();
                    loop {
                        if let Some(assigner) = cur_env.borrow_mut().entries.get_mut(&name) {
                            *assigner = v.clone();
                            break;
                        };
                        let b = cur_env.borrow();
                        let child = match &b.parent {
                            Some(p) => p.clone(),
                            _ => {
                                drop(b);
                                env.borrow_mut().entries.insert(name, v.clone());
                                break;
                            }
                        };
                        drop(b);
                        cur_env = child;
                    }
                }
                Stmt::Procedure(proc) => {
                    // TODO: this clone is wildly inefficient
                    env.borrow_mut().entries.insert(
                        self.source[Into::<std::ops::Range<_>>::into(proc.name)].into(),
                        Value::Procedure(Rc::new(proc.clone())),
                    );
                }
                Stmt::IndexAssign { root, index, value } => {
                    let rootv = tee!(self.eval_expr(root, env.clone()));
                    let Value::Array(rootv) = &rootv else {
						fail!(format!("{rootv:?} is not an array"), root.span());
					};

                    let indexv = tee!(self.eval_expr(index, env.clone()));
                    let Value::Number(idx) = &indexv else {
						fail!(format!("{indexv:?} is not a number"), index.span());
					};

                    let mut rootv = rootv.borrow_mut();
                    let Some(vptr) = rootv.items.get_mut(*idx as u32 as usize - 1) else {
						fail!(format!("index is out of bounds: the length is {:?} but the index is {idx}", rootv.items.len()), stmt.span());
					};

                    *vptr = tee!(self.eval_expr(value, env.clone()));
                }
                Stmt::Return { value, .. } => return self.eval_expr(value, env),
                Stmt::If {
                    cond,
                    scope,
                    else_ifs,
                    els,
                } => 'blk: {
                    let c1 = tee!(self.eval_expr(cond, env.clone()));
                    let Value::Bool(b) = c1 else {
						fail!(format!("{c1:?} is not a boolean"), cond.span());
					};

                    if b {
                        let scope_val = tee!(self.eval_scope(scope, env.clone()));

                        let Value::Void = scope_val else {
                            return scope_val;
                        };

                        break 'blk;
                    }

                    for else_if in else_ifs.iter() {
                        let Value::Bool(b) = tee!(self.eval_expr(&else_if.cond, env.clone())) else {
							panic!();
						};

                        if b {
                            let scope_val = tee!(self.eval_scope(&else_if.scope, env.clone()));

                            let Value::Void = scope_val else {
								return scope_val;
							};

                            break 'blk;
                        }
                    }

                    if let Some(els) = els {
                        let scope_val = tee!(self.eval_scope(els, env.clone()));

                        let Value::Void = scope_val else {
							return scope_val;
						};

                        break 'blk;
                    }
                }
                Stmt::RepeatN { n: n_expr, scope } => {
                    let count = tee!(self.eval_expr(n_expr, env.clone()));

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
                        let val = tee!(self.eval_scope(scope, env.clone()));

                        let Value::Void = val else {
							return val;
						};

                        n -= 1;
                    }
                }
                Stmt::RepeatUntil { cond, scope } => loop {
                    let val = tee!(self.eval_expr(cond, env.clone()));

                    let Value::Bool(b) = val else {
						fail!(format!("{val:?} is not a boolean"), cond.span());
					};

                    if b {
                        break;
                    }

                    let val = tee!(self.eval_scope(scope, env.clone()));
                    let Value::Void = val else {
						return val;
					};
                },
                Stmt::For {
                    alias,
                    array,
                    scope,
                } => {
                    let arr = tee!(self.eval_expr(array, env.clone()));
                    let Value::Array(arr) = &arr else {
						fail!(format!("{:?} is not an array", array), array.span());
					};

                    let mut i = 0;
                    let len = arr.borrow().items.len();

                    loop {
                        if i >= len {
                            break;
                        }

                        let arrb = arr.borrow();
                        let Some(val) = arrb.items.get(i) else {
							break;
						};

                        env.borrow_mut().entries.insert(
                            self.source[alias.start as usize..alias.end as usize].into(),
                            val.clone(),
                        );

                        let scope_val = tee!(self.eval_scope(scope, env.clone()));

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
