import { invoke } from "@tauri-apps/api/core";
import type {
  AiConversation,
  AiMessage,
  AiSettingsPayload,
  AiStatus,
  Bootstrap,
  Category,
  CommandDto,
  CommandOutput,
  GpaDashboard,
  LinkDto,
  ProjectCard,
  RunningTimer,
  SearchHit,
  Settings,
  Tag,
  TimeEntry,
  Todo,
  Unlock,
} from "./types";

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  listCategories: (includeDisabled = true) =>
    invoke<Category[]>("list_categories", { includeDisabled }),
  setCategoryEnabled: (id: string, enabled: boolean) =>
    invoke<Category>("set_category_enabled", { id, enabled }),
  createCustomCategory: (name: string, icon: string, accent: string) =>
    invoke<Category>("create_custom_category", { name, icon, accent }),
  listTags: () => invoke<Tag[]>("list_tags"),
  createCustomTag: (name: string) => invoke<Tag>("create_custom_tag", { name }),
  listProjects: (categoryId: string, includeArchived = false) =>
    invoke<ProjectCard[]>("list_projects", { categoryId, includeArchived }),
  getProject: (id: string) => invoke<ProjectCard>("get_project", { id }),
  createProject: (input: Record<string, unknown>) => invoke<ProjectCard>("create_project", { input }),
  updateProject: (input: Record<string, unknown>) => invoke<ProjectCard>("update_project", { input }),
  runningTimer: () => invoke<RunningTimer | null>("running_timer"),
  startTimer: (projectId: string, topicId?: string) =>
    invoke("start_timer", { projectId, topicId }),
  stopTimer: () => invoke<number>("stop_timer"),
  pauseTimer: () => invoke<number>("pause_timer"),
  listTimeEntries: (projectId: string) => invoke<TimeEntry[]>("list_time_entries", { projectId }),
  addManualEntry: (payload: Record<string, unknown>) => invoke("add_manual_entry", payload),
  updateTimeEntry: (payload: Record<string, unknown>) => invoke("update_time_entry", payload),
  listTodos: (filter: Record<string, unknown>) => invoke<Todo[]>("list_todos", filter),
  createTodo: (payload: Record<string, unknown>) => invoke<Todo>("create_todo", payload),
  setTodoStatus: (id: string, status: string) => invoke("set_todo_status", { id, status }),
  deleteTodo: (id: string) => invoke("delete_todo", { id }),
  todoKinds: (categoryId: string) => invoke<[string, string, string][]>("todo_kinds", { categoryId }),
  listLinks: (projectId: string) => invoke<LinkDto[]>("list_links", { projectId }),
  upsertLink: (link: LinkDto) => invoke<LinkDto>("upsert_link", { link }),
  deleteLink: (id: string) => invoke("delete_link", { id }),
  listCommands: (projectId: string) => invoke<CommandDto[]>("list_commands", { projectId }),
  upsertCommand: (command: CommandDto) => invoke("upsert_command", { command }),
  deleteCommand: (id: string) => invoke("delete_command", { id }),
  runCommand: (payload: Record<string, unknown>) => invoke<CommandOutput>("run_command", payload),
  gitStatus: (projectId: string) => invoke("git_status", { projectId }),
  gitRun: (projectId: string, action: string) => invoke<string>("git_run", { projectId, action }),
  openPath: (path: string) => invoke("open_path", { path }),
  revealPath: (path: string) => invoke("reveal_path", { path }),
  openUrl: (url: string) => invoke("open_url", { url }),
  openVscode: (folder: string) => invoke("open_vscode", { folder }),
  openTerminal: (folder: string) => invoke("open_terminal", { folder }),
  pickFolder: () => invoke<string | null>("pick_folder"),
  pickFile: () => invoke<string | null>("pick_file"),
  listProjectFiles: (projectId: string) => invoke("list_project_files", { projectId }),
  addProjectFile: (payload: Record<string, unknown>) => invoke("add_project_file", payload),
  removeProjectFile: (id: string) => invoke("remove_project_file", { id }),
  scanProjectFolder: (projectId: string) => invoke("scan_project_folder", { projectId }),
  cachedFiles: (projectId: string) => invoke("cached_files", { projectId }),
  listVersions: (projectId: string) => invoke("list_versions", { projectId }),
  addVersion: (payload: Record<string, unknown>) => invoke("add_version", payload),
  listTopics: (projectId: string) => invoke("list_topics", { projectId }),
  upsertTopic: (topic: Record<string, unknown>) => invoke("upsert_topic", { topic }),
  listLessons: (projectId: string) => invoke("list_lessons", { projectId }),
  upsertLesson: (lesson: Record<string, unknown>) => invoke("upsert_lesson", { lesson }),
  listExams: (filter: Record<string, unknown>) => invoke("list_exams", filter),
  upsertExam: (exam: Record<string, unknown>) => invoke("upsert_exam", { exam }),
  universityDashboard: () => invoke<GpaDashboard>("university_dashboard"),
  listActivity: (filter: Record<string, unknown>) => invoke("list_activity", filter),
  listAchievements: (projectId?: string) => invoke<Unlock[]>("list_achievements", { projectId }),
  achievementCatalog: () => invoke("achievement_catalog"),
  globalProgress: () => invoke("global_progress"),
  search: (query: string) => invoke<SearchHit[]>("search", { query }),
  projectContext: (projectId: string) => invoke("project_context", { projectId }),
  aiStatus: () => invoke<AiStatus>("ai_status"),
  aiSaveSettings: (settings: AiSettingsPayload) =>
    invoke<AiSettingsPayload>("ai_save_settings", { settings }),
  aiSetKey: (key: string) => invoke<AiStatus>("ai_set_key", { key }),
  aiClearKey: () => invoke<AiStatus>("ai_clear_key"),
  aiRevealKey: () => invoke<string>("ai_reveal_key"),
  aiTestConnection: () => invoke<{ ok: boolean; model: string; fallbackUsed: boolean; text: string }>("ai_test_connection"),
  aiChatStream: (input: { conversationId?: string | null; projectId?: string; message: string }) =>
    invoke<AiMessage>("ai_chat_stream", { input }),
  aiListConversations: (projectId?: string) =>
    invoke<AiConversation[]>("ai_list_conversations", { projectId }),
  aiListMessages: (conversationId: string) =>
    invoke<AiMessage[]>("ai_list_messages", { conversationId }),
  aiNewConversation: (projectId?: string) => invoke<string>("ai_new_conversation", { projectId }),
  aiClearConversation: (conversationId: string) => invoke("ai_clear_conversation", { conversationId }),
  aiInterpretSearch: (query: string) => invoke<{
    hits: SearchHit[];
    ai: boolean;
    interpretation?: { projectIds: string[]; explanation: string } | null;
    fallbackUsed?: boolean;
  }>("ai_interpret_search", { query }),
  aiPrioritizeTodos: (projectId: string) => invoke("ai_prioritize_todos", { projectId }),
  aiApplyTodoPriorities: (order: string[]) => invoke("ai_apply_todo_priorities", { order }),
  aiGeneratePrompt: (projectId: string, extra?: string) => invoke("ai_generate_prompt", { projectId, extra }),
  aiSummarizeChangelog: (projectId: string, versionId?: string) =>
    invoke<string>("ai_summarize_changelog", { projectId, versionId }),
  aiGenerateChangelog: (projectId: string) => invoke("ai_generate_changelog", { projectId }),
  aiTodosFromChangelog: (projectId: string, changelog: string) =>
    invoke("ai_todos_from_changelog", { projectId, changelog }),
  aiSuggestCommit: (projectId: string) => invoke("ai_suggest_commit", { projectId }),
  gitLog: (projectId: string) => invoke<string[]>("git_log", { projectId }),
  gitDiff: (projectId: string) => invoke("git_diff", { projectId }),
  listDocumentRecords: (projectId: string) => invoke("list_document_records", { projectId }),
  exportData: () => invoke("export_data"),
  importData: (data: unknown) => invoke("import_data", { data }),
  enqueueIndexJob: (projectId: string) => invoke<string>("enqueue_index_job", { projectId }),
  pollJobs: () => invoke<{ pending: number }>("poll_jobs"),
  completeOnboarding: (defaultWorkspace?: string) =>
    invoke("complete_onboarding", { defaultWorkspace }),
  syncStatus: () => invoke("sync_status"),
  pushSync: () => invoke("push_sync"),
  checkForUpdates: () => invoke("check_for_updates"),
  localVersion: () => invoke<string>("local_version"),
  authRegister: (email: string, password: string, displayName?: string) =>
    invoke("auth_register", { email, password, displayName }),
  authLogin: (email: string, password: string) => invoke("auth_login", { email, password }),
  authLogout: () => invoke("auth_logout"),
  authForgot: (email: string) => invoke("auth_forgot", { email }),
};

export function formatError(err: unknown) {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) return String((err as Error).message);
  return "Something went wrong";
}
