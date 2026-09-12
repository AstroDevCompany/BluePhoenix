# Contributing to BluePhoenix

Thank you for being interested in contributing to **BluePhoenix**.

BluePhoenix is built to help people organize, track and actually make progress on the things they work on.

Contributions are welcome, but please understand the project's licensing model before contributing.

---

## ⚠️ Important: read the license first

BluePhoenix is **source-available**, not unrestricted open-source software.

The source code is publicly visible so that people can:

- inspect it
- learn from it
- build it themselves
- modify their own copy
- contribute improvements
- experiment with the project

However, the BluePhoenix license does **not** grant permission to redistribute, publish, sublicense, or commercially sell BluePhoenix or modified versions of it without explicit permission from the copyright owner.

Read the complete:

**[LICENSE.md](LICENSE.md)**

before contributing.

---

# Getting started

## 1. Fork the repository

Create your own fork of the BluePhoenix repository.

## 2. Clone your fork

```bash
git clone <your-fork-url>
cd BluePhoenix
```

## 3. Create a branch

Use a descriptive branch name:

```bash
git checkout -b feature/project-achievements
```

Examples:

```text
feature/university-course-dashboard
feature/github-integration
feature/achievement-tooltip
fix/project-sync
fix/windows-folder-opening
docs/development-setup
```

## 4. Install dependencies

Follow the development setup documented in the repository.

Do not commit generated dependencies, secrets, local databases, build output or machine-specific configuration.

---

# What you can contribute

### Bug fixes

Fix incorrect behavior, crashes, broken synchronization, UI problems, platform-specific issues, etc.

### Features

Add functionality that fits the BluePhoenix architecture and product direction.

For larger features, **open an issue before implementing them**.

### UI/UX

BluePhoenix places significant emphasis on its interface.

Contributions should respect the existing design system.

Avoid introducing:

- default browser controls
- unrelated component libraries
- inconsistent icon sets
- arbitrary colors
- unnecessary animations
- inconsistent spacing
- platform-specific UI when a cross-platform solution exists

### Documentation

Documentation improvements are always welcome.

### Platform support

macOS and Windows are primary targets.

Platform-specific fixes are particularly useful when they improve behavior without damaging the other platform.

---

# Architecture principles

## Local-first

Local functionality should work without requiring an active internet connection wherever reasonably possible.

Cloud synchronization should complement local functionality rather than becoming a hard dependency for basic application usage.

## Universal foundation

Projects should share a common foundation.

Universal capabilities include concepts such as:

- projects
- categories
- time entries
- TODOs
- links
- files
- XP
- achievements
- activity

Category-specific capabilities should be layered on top.

```text
Universal Project
        │
        ├── Software capabilities
        │   ├── Git
        │   ├── GitHub
        │   ├── Commands
        │   └── Versions
        │
        └── University capabilities
            ├── Courses
            ├── Topics
            ├── Exams
            ├── Attendance
            └── Grades
```

Avoid creating one enormous object containing hundreds of nullable fields.

## Capability-driven UI

Avoid scattering checks such as:

```ts
if (category === "software") {
    ...
}
```

throughout the entire application.

Use a capability-oriented architecture so future categories can be added without rewriting the core application.

---

# Achievement contributions

Achievements are an important part of BluePhoenix.

When adding an achievement:

1. Give it a unique ID.
2. Define its condition separately from its UI.
3. Add a placeholder PNG asset.
4. Register the achievement in the centralized achievement system.
5. Add it to the achievement documentation.
6. Clearly document the PNG filename so it can later be replaced with final artwork.

Do **not** use emojis as achievement artwork.

---

# UI contributions

Every interactive UI component should use the BluePhoenix design system.

This includes:

- buttons
- inputs
- selects
- dropdowns
- dialogs
- tooltips
- popovers
- tabs
- menus
- scrollbars
- notifications
- command palettes
- achievement tooltips

Do not introduce browser-default UI unless native OS functionality is explicitly required.

For ordinary interface icons, use the project's established icon library.

Achievement artwork is separate and should use dedicated PNG assets.

---

# Achievement tooltips

Achievement tooltips intentionally have a more expressive visual style than ordinary tooltips.

They should preserve:

- accessibility
- keyboard support
- viewport collision handling
- reduced-motion support
- good performance

The animated rainbow/iridescent treatment should remain special to achievements rather than becoming a generic application-wide effect.

---

# Code quality

Before submitting a pull request:

- format your code
- run linting
- run relevant tests
- verify the application builds
- test affected functionality
- check both macOS and Windows behavior where applicable
- remove debugging code
- remove unnecessary console logs
- make sure no secrets or credentials are committed

Keep changes focused.

---

# Commits

Use clear commit messages.

Prefer:

```text
feat: add university exam tracking
fix: prevent duplicate time entries
feat: add achievement tooltip
fix: improve Windows folder opening
docs: update achievement documentation
```

Avoid:

```text
stuff
changes
update
final
test
asdfgh
```

---

# Pull requests

A pull request should explain:

### What changed?

Briefly describe the implementation.

### Why?

Explain the problem or motivation.

### How was it tested?

List relevant platforms and test cases.

### Screenshots

For UI changes, include screenshots or a short recording when useful.

---

# Pull request checklist

Before opening a PR:

- [ ] The change is focused and intentional.
- [ ] Existing functionality still works.
- [ ] The project builds successfully.
- [ ] Relevant tests pass.
- [ ] UI follows the BluePhoenix design system.
- [ ] No browser-default UI was introduced accidentally.
- [ ] No secrets are included.
- [ ] New achievements include placeholder artwork.
- [ ] Achievement assets are documented.
- [ ] Documentation was updated where necessary.
- [ ] Platform-specific behavior was considered.
- [ ] The change does not unnecessarily expand the scope of the PR.

---

# Issues

Before opening an issue, check whether the problem has already been reported.

A good bug report should include:

- operating system
- BluePhoenix version/commit
- steps to reproduce
- expected behavior
- actual behavior
- screenshots or logs where useful

For feature requests, explain the problem you are trying to solve rather than only proposing a specific implementation.

---

# Design philosophy

BluePhoenix is intended to feel:

**focused · premium · fast · motivating · personal**

It should not feel like:

**a generic admin dashboard · a spreadsheet · a browser form · a toy productivity game**

When making design decisions, prioritize the first list.

---

# Questions

If you are unsure whether a contribution fits the project, open an issue or discussion before spending significant time implementing it.

---

## Thank you

Whether you submit a one-line fix, improve the documentation, redesign a component or contribute a major feature:

**thank you for helping BluePhoenix rise.**
