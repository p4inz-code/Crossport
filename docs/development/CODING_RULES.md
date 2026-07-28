# CrossPort Coding Rules

Version: 1.0

---

# General Rules

Code must prioritize:

- Readability
- Reliability
- Maintainability

---

# Architecture Rules

Never:

- Put business logic in UI
- Mix platform-specific code with shared logic
- Create unnecessary dependencies
- Ignore module boundaries

---

# Type Safety

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

# Error Handling

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

# AI Development Rules

AI-generated code must:

- Follow architecture
- Include tests
- Avoid shortcuts
- Explain significant decisions

AI is a tool, not the architect.

---

# Principle

Clean architecture beats fast implementation.