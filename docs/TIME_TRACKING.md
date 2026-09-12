# Time tracking

Time is stored as **start and end timestamps**, never as a JavaScript counter.

- Start, pause, and stop persist immediately to SQLite.
- After sleep, crash, or restart, a running timer is recovered as `now - started_at` (never negative).
- Only one open timer exists at a time. If another device owns it, you will see that it is running elsewhere.

Terminology changes by workspace: development time, study time, training time, or time worked.

University study sessions can be attached to a **topic**. Attendance lessons are a different record and must not be confused with study time.

Add a manual entry when you forgot to start the timer. You can delete entries; project XP is recomputed from remaining timestamps (1 minute = 1 XP).
