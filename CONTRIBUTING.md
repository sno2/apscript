# Contributing Guide

## Parser

- When adding any new calls to functions that return `Value`, make sure you wrap
  it in a `tee!(..)` to make sure that we handle exceptions correctly.
