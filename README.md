# 🔥 BluePhoenix

### Your projects. Your progress. Your command center.

[![Status](https://img.shields.io/badge/status-in%20development-orange)](#)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-blue)](#)
[![License](https://img.shields.io/badge/license-BluePhoenix%20Source--Available-red)](LICENSE.md)

> **Stop losing track of what you're building.**
>
> BluePhoenix is a desktop command center for the things you actually work on — software projects, university courses, creative work, fitness goals, personal projects, and everything in between.

**[Visit BluePhoenix →](https://lucashots.com)**

---

## The idea

Most productivity tools treat everything as a task.

BluePhoenix treats everything as a **project**.

A software project isn't just a list of TODOs.

A university course isn't just a calendar event.

A photography project isn't just a folder.

BluePhoenix brings the things that actually matter together:

**time · progress · files · links · tasks · milestones · achievements · history**

And it does it without forcing every kind of project into the same generic template.

---

## ⚡ Built for people who build things

### 💻 Software

Connect your development workflow directly to your project.

- GitHub repository integration
- Git actions
- Pull from Git
- Open in VS Code
- Open terminal
- Open project folder
- Technology and framework tags
- Custom commands
- Pinned commands
- Versions and changelogs
- Release tracking
- Development time tracking
- Project TODOs
- Development milestones

Your project dashboard becomes the place you go **before, during, and after coding**.

### 🎓 University

Turn every course into something you can actually measure.

- Course documents and PDFs
- Course topics
- Study-time tracking
- University attendance tracking
- Lesson tracking
- Exam attempts
- Failed and passed attempts
- Grades
- Final grades
- CFU tracking
- Weighted average / GPA calculation
- Course TODOs
- University links
- Study milestones

### 📸 Creative & personal projects

Not everything has a Git repository.

BluePhoenix isn't designed around software.

Create projects for:

- Photography
- Fitness
- Studying
- Personal goals
- Research
- Writing
- Creative work
- Custom categories

Connect folders, files, links and whatever else matters to you.

---

# 🏆 Build progress. Earn achievements.

Work on your projects.

Track your time.

Complete milestones.

Earn achievements.

Every project can build its own history of accomplishments, represented through dedicated visual trophies and medals.

10 hours.

25 hours.

50 hours.

100 hours.

First release.

Major milestone.

First completed course.

And much more.

The goal isn't to turn productivity into a game.

It's to make progress **visible**.

Because sometimes seeing that you've already put 50 hours into something is exactly what makes you want to put in the next 10.

---

# 🧭 One command center. Different worlds.

BluePhoenix uses category-specific workspaces instead of forcing everything into one giant dashboard.

```text
BLUEPHOENIX
│
├── Software
│   ├── Projects
│   ├── TODOs
│   └── Commands
│
├── University
│   ├── Courses
│   ├── TODOs
│   └── Exams
│
├── Photography
│   └── Projects
│
└── Personal
    └── Projects
```

Software stays software.

University stays university.

Your workspace adapts to what you're actually doing.

And if you don't need a category, disable it.

---

# 🖥️ Desktop-first

BluePhoenix is designed as a proper desktop application rather than a website pretending to be one.

The current target is:

- macOS
- Windows

The architecture is designed with future mobile expansion in mind.

---

# 🔒 Local-first

Your projects should remain useful even when you're offline.

BluePhoenix is designed around a local-first architecture:

```text
          ┌────────────────────┐
          │     BluePhoenix    │
          │      Desktop       │
          └─────────┬──────────┘
                    │
             Local database
                    │
          ┌─────────▼──────────┐
          │   Local project    │
          │       state        │
          └─────────┬──────────┘
                    │
              Async sync
                    │
          ┌─────────▼──────────┐
          │      Cloud         │
          │  authentication    │
          │   + synchronization│
          └────────────────────┘
```

The application should remain functional without an internet connection wherever possible, while cloud synchronization keeps your data available across devices.

Local files remain local by default.

---

# 🛠️ Technology

BluePhoenix is being built with a modern desktop stack:

- **Tauri 2**
- **React**
- **TypeScript**
- **Rust**
- **Vite**
- **SQLite**
- **PostgreSQL / Neon**
- **GitHub integration**

---

# 🚧 Status

BluePhoenix is currently in active development.

The architecture and feature set are evolving.

Expect things to change.

Expect things to break.

Expect the phoenix to catch fire occasionally.

That's part of the process.

---

# 🚀 Development

Clone the repository:

```bash
git clone <repository-url>
cd BluePhoenix
```

Install dependencies and follow the development setup described in the project documentation.

---

# 🤝 Contributing

BluePhoenix is source-available and welcomes contributions.

You can:

- report bugs
- suggest improvements
- improve documentation
- submit fixes
- develop features
- experiment with the code
- build BluePhoenix locally for yourself

Before contributing, read:

**[CONTRIBUTING.md](CONTRIBUTING.md)**

Please also read:

**[LICENSE.md](LICENSE.md)**

The project is intentionally **not licensed for unrestricted redistribution or commercial resale**.

---

# 🌐 Learn more

The BluePhoenix project website is currently hosted at:

**[lucashots.com](https://lucashots.com)**

The website will eventually become the central place for:

- downloads
- documentation
- feature information
- screenshots
- changelogs
- project news
- future releases

---

# 🔥 Why BluePhoenix?

Because projects die quietly.

You stop working on them.

You forget why you started.

The files remain somewhere on your computer.

The Git repository remains untouched.

The TODO list becomes irrelevant.

And eventually, the project becomes another abandoned folder.

BluePhoenix is built around a simple idea:

> **Make your work visible enough that you want to keep going.**

Every hour.

Every task.

Every release.

Every milestone.

Every trophy.

**Rise again. Build again.**

# BluePhoenix

### Build. Track. Progress. Rise.

## Requirements

- Rust 1.77+
- Node 20+
- pnpm
- Windows 10/11 or macOS 12+

## Desktop

From the repo root:

```bash
pnpm install
pnpm desktop:dev
```

Or from the desktop app:

```bash
cd apps/desktop
pnpm tauri dev
```

`npm run tauri` only exists on `apps/desktop`, not at the repo root. Use `pnpm desktop:dev` or `cd apps/desktop` first.

Local data: `%APPDATA%/BluePhoenix/bluephoenix.sqlite` on Windows, `~/Library/Application Support/BluePhoenix/` on macOS.

## Tests

```bash
cargo test -p bluephoenix-domain
```

## API (optional cloud)

The desktop app runs fully offline. Cloud sync is user-scoped metadata only.

```bash
cp apps/api/.env.example apps/api/.env
# set DATABASE_URL to a Neon or local Postgres instance
cargo run -p bluephoenix-api
```

Never put Neon credentials in the desktop binary. The client talks to `/v1` with a JWT.

Sensitive keys (future OpenRouter) use a separate encrypted-secret collection, not the metadata outbox.

## Packaging

```bash
cd apps/desktop
pnpm tauri build
```

Replace the updater public key in `apps/desktop/src-tauri/tauri.conf.json` before shipping. The RAW version file is `version.json` at the repository root; the URL is configurable in Settings → Updates.

## Docs

- [Using BluePhoenix](docs/USER_GUIDE.md)
- [Workspaces](docs/WORKSPACES.md)
- [Time tracking](docs/TIME_TRACKING.md)
- [University & GPA](docs/UNIVERSITY.md)
- [Sync & accounts](docs/SYNC.md)
- [Updates](docs/UPDATES.md)
- [Achievement assets](docs/ACHIEVEMENTS.md)
- [Architecture](docs/ARCHITECTURE.md)
