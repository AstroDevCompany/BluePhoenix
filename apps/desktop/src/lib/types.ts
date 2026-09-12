export type CapabilitySet = {
  git: boolean;
  github: boolean;
  developmentCommands: boolean;
  languages: boolean;
  frameworks: boolean;
  vscode: boolean;
  terminal: boolean;
  versions: boolean;
  files: boolean;
  links: boolean;
  todos: boolean;
  timeTracking: boolean;
  studyTracking: boolean;
  attendance: boolean;
  exams: boolean;
  grades: boolean;
  topics: boolean;
  primaryFile: boolean;
  xp: boolean;
  achievements: boolean;
  activity: boolean;
};

export type Category = {
  id: string;
  slug: string;
  kind: "software" | "university" | "generic" | string;
  name: string;
  icon: string;
  accent: string;
  enabled: boolean;
  sortOrder: number;
  isSystem: boolean;
  capabilities: CapabilitySet;
  terminology: {
    workspace: string;
    itemSingular: string;
    itemPlural: string;
    timeLabel: string;
    timerStart: string;
  };
  nav: { id: string; label: string; route: string; capability?: string | null }[];
};

export type Tag = { id: string; name: string; kind: string };

export type CommandDto = {
  id: string;
  projectId: string;
  name: string;
  command: string;
  description: string;
  workingDirectory?: string | null;
  pinned: boolean;
  favorite: boolean;
  sortOrder: number;
  dangerous: boolean;
};

export type Unlock = {
  id: string;
  achievementId: string;
  name: string;
  description: string;
  icon: string;
  rarity: string;
  earnedAt: string;
  projectId?: string | null;
};

export type GitStatus = {
  available: boolean;
  isRepo: boolean;
  branch?: string | null;
  ahead: number;
  behind: number;
  dirty: boolean;
  lastCommit?: string | null;
  lastCommitAt?: string | null;
  error?: string | null;
};

export type ProjectCard = {
  id: string;
  categoryId: string;
  kind: string;
  name: string;
  description: string;
  status: string;
  favorite: boolean;
  archived: boolean;
  totalTrackedSeconds: number;
  xp: number;
  level: number;
  levelRatio: number;
  githubUrl?: string | null;
  websiteUrl?: string | null;
  currentVersion?: string | null;
  cfu?: number | null;
  finalGrade?: number | null;
  averageGrade?: number | null;
  honors: boolean;
  openTodos: number;
  pinnedCommand?: CommandDto | null;
  tags: Tag[];
  achievements: Unlock[];
  localPath?: string | null;
  primaryFilePath?: string | null;
  attendance?: { lessons: number; attended: number; missed: number; percent: number; seconds: number } | null;
  lastActivity?: string | null;
  git?: GitStatus | null;
};

export type Settings = {
  theme: string;
  accent: string;
  reducedMotion: boolean;
  defaultWorkspace?: string | null;
  automaticUpdates: boolean;
  updateChannel: string;
  rawVersionUrl: string;
  gitPath: string;
  vscodePath: string;
  terminal: string;
  shell: string;
  apiBase: string;
  includeFailedGrades: boolean;
  onboardingComplete: boolean;
};

export type AiStatus = {
  enabled: boolean;
  hasKey: boolean;
  maskedKey?: string | null;
  models: string[];
  commitFollowStyle: boolean;
  setupDismissed: boolean;
  signedIn: boolean;
  cloudSecret: boolean;
};

export type AiSettingsPayload = {
  enabled: boolean;
  models: string[];
  commitFollowStyle: boolean;
  setupDismissed: boolean;
};

export type AiMessage = {
  id: string;
  conversationId: string;
  role: string;
  content: string;
  model?: string | null;
  fallbackUsed: boolean;
  createdAt: string;
};

export type AiConversation = {
  id: string;
  projectId?: string | null;
  title: string;
  createdAt: string;
  updatedAt: string;
};

export type Bootstrap = {
  deviceId: string;
  settings: Settings;
  categories: Category[];
  tags: Tag[];
  sync: { status: string; label: string; pending: number; lastError?: string | null };
  onboardingComplete: boolean;
  user?: { id: string; email?: string | null; displayName?: string | null } | null;
};

export type Todo = {
  id: string;
  projectId: string;
  projectName: string;
  categoryId: string;
  kindId?: string | null;
  kindName?: string | null;
  title: string;
  description: string;
  priority: string;
  status: string;
  dueDate?: string | null;
  createdAt: string;
  completedAt?: string | null;
};

export type LinkDto = {
  id: string;
  projectId: string;
  title: string;
  url: string;
  icon?: string | null;
  description?: string | null;
  pinned: boolean;
  sortOrder: number;
};

export type TimeEntry = {
  id: string;
  projectId: string;
  topicId?: string | null;
  topicTitle?: string | null;
  deviceId: string;
  startedAt: string;
  endedAt?: string | null;
  seconds: number;
  notes?: string | null;
};

export type RunningTimer = {
  entry: TimeEntry;
  projectName: string;
  deviceId: string;
  isLocalDevice: boolean;
};

export type CommandOutput = {
  code: number;
  stdout: string;
  stderr: string;
};

export type GpaDashboard = {
  simpleAverage?: number | null;
  weightedAverage?: number | null;
  usedWeighted: boolean;
  included: string[];
  excludedMissingFinal: string[];
  excludedFailed: string[];
  totalCfu: number;
  completedCfu: number;
  passedCourses: number;
  failedAttempts: number;
  totalAttempts: number;
  studySeconds: number;
  attendancePercent: number;
  maxGrade: number;
};

export type SearchHit = {
  title: string;
  subtitle: string;
  entityType: string;
  entityId: string;
  categoryId?: string | null;
  projectId?: string | null;
};

export type ActivityEvent = {
  id: string;
  eventType: string;
  createdAt: string;
  projectName?: string | null;
  categoryId?: string | null;
};

export type Caps = CapabilitySet;

export function hasCap(category: Category | undefined, key: keyof CapabilitySet) {
  return Boolean(category?.capabilities?.[key]);
}
