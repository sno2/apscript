#[cfg(feature = "js")]
use std::fmt::Write as WriteFmt;
#[cfg(not(feature = "js"))]
use std::io::{StdoutLock, Write};

#[cfg(not(feature = "js"))]
use gc::Gc;
#[cfg(not(feature = "js"))]
use rand::Rng;

use crate::{
    fail, tee,
    vm::{Builtin, BuiltinPtr, Value, VM},
};

pub fn inject(vm: &mut VM) {
    let builtins = [
        ("DISPLAY", display as BuiltinPtr),
        #[cfg(not(feature = "js"))]
        ("INPUT", input),
        ("RANDOM", random),
        ("APPEND", append),
        ("INSERT", insert),
        ("REMOVE", remove),
        ("LENGTH", length),
    ];

    vm.scope.extend(
        builtins
            .into_iter()
            .map(|(name, ptr)| (name, Value::Builtin(Builtin(ptr)))),
    );
}

#[cfg(not(feature = "js"))]
fn display_helper(stdout: &mut StdoutLock, args: &[Value]) -> Value {
    let mut iter = args.into_iter();
    if let Some(arg0) = iter.next() {
        let Ok(_) = write!(stdout, "{}", arg0) else {
			fail!("failed to write to stdout", BUILTIN);
		};
        for arg in iter {
            let Ok(_) = write!(stdout, " {}", arg) else {
				fail!("failed to write to stdout", BUILTIN);
			};
        }
    }
    Value::Void
}

#[cfg(feature = "js")]
fn display_helper(out: &mut String, args: &[Value]) -> Value {
    let mut iter = args.into_iter();
    if let Some(arg0) = iter.next() {
        write!(out, "{}", arg0).unwrap();
        for arg in iter {
            write!(out, " {}", arg).unwrap();
        }
    }
    Value::Void
}

fn validate_index(idx: f32, out: &mut usize) -> Value {
    if idx.floor() != idx {
        fail!("array index is not an integer", BUILTIN);
    }

    if idx < 1. {
        fail!("array index out of range", BUILTIN);
    }

    *out = idx as usize;
    Value::Void
}

#[cfg(not(feature = "js"))]
fn display(_: &mut VM, args: &[Value]) -> Value {
    let mut stdout = std::io::stdout().lock();
    _ = tee!(display_helper(&mut stdout, args));
    let Ok(_) = write!(stdout, "\n") else {
		fail!("failed to write to stdout", BUILTIN);
	};
    let Ok(_) = stdout.flush() else {
		fail!("failed to flush stdout", BUILTIN);
	};
    Value::Void
}

#[cfg(feature = "js")]
fn display(_: &mut VM, args: &[Value]) -> Value {
    let mut out = String::new();
    _ = tee!(display_helper(&mut out, args));
    web_sys::console::log_1(&out.as_str().into());
    Value::Void
}

#[cfg(not(feature = "js"))]
fn input(_: &mut VM, args: &[Value]) -> Value {
    let mut stdout = std::io::stdout().lock();

    if args.len() == 0 {
        let Ok(_) = write!(stdout, "Input: ") else {
			fail!("failed to write to stdout", BUILTIN);
		};
    } else {
        _ = tee!(display_helper(&mut stdout, args));
        let Ok(_) = write!(stdout, " ") else {
			fail!("failed to write to stdout", BUILTIN);
		};
    }

    let Ok(_) = stdout.flush() else {
		fail!("failed to flush stdout", BUILTIN);
	};

    let stdin = std::io::stdin();

    let mut out = String::new();
    let Ok(_) = stdin.read_line(&mut out) else {
		fail!("failed to read line from stdout", BUILTIN);
	};

    let outs = out.trim();

    if let Ok(f) = outs.parse() {
        Value::Number(f)
    } else {
        Value::String(Gc::new(outs.to_owned()))
    }
}

#[cfg(not(feature = "js"))]
fn random(vm: &mut VM, args: &[Value]) -> Value {
    let rng = vm.rng.get_or_insert_with(rand::thread_rng);

    match (args.get(0), args.get(1)) {
        (Some(Value::Number(n1)), Some(Value::Number(n2))) => {
            Value::Number(rng.gen_range(n1.round() as i32..=n2.round() as i32) as f32)
        }
        _ => fail!("expected valid range start and end numbers", BUILTIN),
    }
}

#[cfg(feature = "js")]
fn random(_: &mut VM, args: &[Value]) -> Value {
    let float = js_sys::Math::random() as f32;

    match (args.get(0), args.get(1)) {
        (Some(Value::Number(n1)), Some(Value::Number(n2))) => {
            Value::Number((float * (n2 - n1 + 1.)).floor() + n1)
        }
        (n1, _) => {
            let Some(Value::Number(_)) = n1 else {
                fail!("expected a number for the first argument", BUILTIN);
            };

            fail!("expected a number for the second argument", BUILTIN);
        }
    }
}

fn append(_: &mut VM, args: &[Value]) -> Value {
    let Some(Value::Array(array)) = args.get(0) else {
		fail!("expected array for the first argument", BUILTIN);
	};

    let Some(val) = args.get(1) else {
		fail!("expected value for the second argument", BUILTIN);
	};

    array.borrow_mut().items.push(val.clone());

    Value::Void
}

fn insert(_: &mut VM, args: &[Value]) -> Value {
    let Some(Value::Array(array)) = args.get(0) else {
		fail!("expected array for the first argument", BUILTIN);
	};

    let Some(Value::Number( idx)) = args.get(1) else {
		fail!("expected index for the second argument", BUILTIN);
	};

    let Some(val) = args.get(2) else {
		fail!("expected value for the third argument", BUILTIN);
	};

    let mut correct_idx = 0;
    _ = tee!(validate_index(*idx, &mut correct_idx));

    let items = &mut array.borrow_mut().items;

    if correct_idx > items.len() {
        fail!("array index out of range", BUILTIN);
    }

    items.insert(correct_idx - 1, val.clone());

    Value::Void
}

fn remove(_: &mut VM, args: &[Value]) -> Value {
    let Some(Value::Array(array)) = args.get(0) else {
		fail!("expected array for the first argument", BUILTIN);
	};

    let Some(Value::Number(idx)) = args.get(1) else {
		fail!("expected number for the second argument", BUILTIN);
	};

    let mut correct_idx = 0;
    _ = tee!(validate_index(*idx, &mut correct_idx));

    let items = &mut array.borrow_mut().items;

    if correct_idx > items.len() {
        fail!("array index out of range", BUILTIN);
    }

    items.remove(correct_idx - 1);

    Value::Void
}

fn length(_: &mut VM, args: &[Value]) -> Value {
    let Some(Value::Array(array)) = args.get(0) else {
		fail!("expected the first argument to be an array", BUILTIN);
	};

    Value::Number(array.borrow().items.len() as f32)
}
