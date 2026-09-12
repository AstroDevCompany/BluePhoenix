# BluePhoenix Achievements

Trophy artwork lives in `apps/desktop/src/assets/achievements/` and is copied to `apps/desktop/public/assets/achievements/` for runtime loading.

Replace a PNG in **both** folders (or re-run `python scripts/generate_assets.py` after updating the generator) using the **same filename**. Achievement IDs, conditions, and rarity live in `crates/domain/src/achievements.rs`. Changing a PNG never requires changing logic.

| Achievement ID | Achievement | Asset File | Trigger |
| --- | --- | --- | --- |
| `first-project` | First Project | `trophy-first-project.png` | First project/course created |
| `hours-1` | First Hour | `trophy-1-hour.png` | 1h tracked on a project |
| `hours-10` | 10 Hours | `trophy-10-hours.png` | 10h project time |
| `hours-25` | 25 Hours | `trophy-25-hours.png` | 25h project time |
| `hours-50` | 50 Hours | `trophy-50-hours.png` | 50h project time |
| `hours-100` | 100 Hours | `trophy-100-hours.png` | 100h project time |
| `todos-10` | 10 TODOs | `trophy-10-todos.png` | 10 completed TODOs |
| `todos-25` | 25 TODOs | `trophy-25-todos.png` | 25 completed TODOs |
| `todos-100` | 100 TODOs | `trophy-100-todos.png` | 100 completed TODOs |
| `first-release` | First Release | `trophy-first-release.png` | First software version |
| `version-1-0` | Version 1.0 | `trophy-first-version.png` | Current version ≥ 1.0.0 |
| `commits-100` | 100 Commits | `trophy-100-commits.png` | 100 git commits |
| `first-website` | First Public Website | `trophy-first-website.png` | Website URL set |
| `first-github` | Source Connected | `trophy-first-github.png` | GitHub URL set |
| `first-course-completed` | First Course Completed | `trophy-first-course.png` | A course marked completed |
| `first-exam-passed` | First Exam Passed | `trophy-first-exam.png` | An exam marked passed |
| `perfect-exam` | Excellent Exam | `trophy-perfect-exam.png` | Passed at max grade (30/30) |
| `study-hours-10` | 10 Study Hours | `trophy-10-study-hours.png` | 10h study on a course |
| `study-hours-100` | 100 Study Hours | `trophy-100-study-hours.png` | 100h study on a course |
| `courses-10` | 10 Courses | `trophy-10-courses.png` | 10 university courses |
| `cfu-30` | 30 CFU | `trophy-30-cfu.png` | 30 completed CFU |
| `cfu-60` | 60 CFU | `trophy-60-cfu.png` | 60 completed CFU |
| `cfu-100` | 100 CFU | `trophy-100-cfu.png` | 100 completed CFU |
| `lessons-10` | 10 Lessons | `trophy-10-lessons.png` | 10 attended lessons |

When you add an achievement: register it in `ACHIEVEMENTS`, add a PNG with a unique filename, and update this table.
