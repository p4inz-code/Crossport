# CrossPort coding rules

Version: 1.0

---

# General rules

Code must prioritize:

- Readability
- Reliability
- Maintainability

---

# Architecture rules

Never:

- Put business logic in UI
- Mix platform-specific code with shared logic
- Create unnecessary dependencies
- Ignore module boundaries

---

# Type safety

Avoid:

- Any types without reason
- Hidden assumptions
- Unsafe conversions

---

# Testing

Every major feature requires:

- Unit tests
- Integration tests where needed

---

# Error handling

Never silently fail.

Errors must be:

- Captured
- Explained
- Recoverable where possible

---

# Dependencies

Every dependency must justify:

- Why it exists
- Maintenance quality
- Long-term value

---

# AI development rules

AI-generated code must:

- Follow architecture
- Include tests
- Avoid shortcuts
- Explain significant decisions

AI is a tool, not the architect.

---

# Principle

Clean architecture beats fast implementation.