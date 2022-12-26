# Contributing Guide

## Parser

- When adding any new calls to functions that return `Value`, make sure you wrap
  it in a `tee!(..)` to make sure that we handle exceptions correctly.
- When calling `eval_scope()` make sure you handle if the returned value is not
  `Value::Void` because that means there was a return statement.
