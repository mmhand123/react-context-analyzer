# Next up

- Figure out the best way to add children into the render graph. We're doing nothing with them right now, but we need a way to
  go from Child -> Parent (starting at children) -> Keep walking up.
- Resolve hooks similarly to components. Need to be able to walk down this one though... component -> hooks -> any hooks they call -> etc
- See if we can answer the question "what are all the components that require providers, and what providers do they require?"
