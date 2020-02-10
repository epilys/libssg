---
title: hello there.
author: epilys
date: Feb 09, 2020
---

An example page.

## what is usable now

This example is usable.

Rendering [links](/#).

Rendering code examples:
```rust
pub trait Component {
    fn draw(&mut self, grid: &mut CellBuffer, area: Area, context: &mut Context);
    fn process_event(&mut self, event: &mut UIEvent, context: &mut Context) -> bool;
    fn is_dirty(&self) -> bool;
    fn set_dirty(&mut self);

		/* ... */
}
```

Rendering lists:

- aba hubba
- aba hubba
- aba hubba
- aba hubba
