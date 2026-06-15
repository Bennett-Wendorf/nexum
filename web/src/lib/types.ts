// Plan types
export interface Plan {
  id: string;
  name: string;
  status: string;
  created: string;
  branch: string;
  goal: string;
  scope: string;
  background: string;
  tasks: TaskRef[];
}

export interface TaskRef {
  id: string;
  name: string;
  completed: boolean;
}

export interface CreatePlanRequest {
  name: string;
  branch: string;
  goal: string;
  scope?: string;
  background?: string;
}

export interface UpdatePlanRequest {
  name?: string;
  goal?: string;
  scope?: string;
  background?: string;
}

export interface TransitionPlanStatusRequest {
  status: string;
  by?: string;
}

// Task types
export interface Task {
  id: string;
  name: string;
  parent_plan: string;
  dependencies: string[];
  description: string;
  acceptance_criteria: string[];
  files_to_modify: string[];
  background: string;
  notes: string;
  status: TaskStatus;
}

export interface TaskStatus {
  id: string;
  status: string;
  agent?: AgentLease;
  transitions: StatusTransition[];
  started_at?: string;
  completed_at?: string;
  attempts: number;
  dependencies: string[];
  dependent_tasks: string[];
  heartbeat_at?: string;
}

export interface AgentLease {
  role: string;
  pid: number;
  leased_at: string;
}

export interface StatusTransition {
  from: string;
  to: string;
  at: string;
  by: string;
}

export interface CreateTaskRequest {
  name: string;
  parent_plan: string;
  dependencies: string[];
  description: string;
  acceptance_criteria: string[];
  files_to_modify: string[];
  background?: string;
  notes?: string;
}

export interface UpdateTaskRequest {
  name?: string;
  description?: string;
  acceptance_criteria?: string[];
  files_to_modify?: string[];
  background?: string;
  notes?: string;
}

export interface TransitionTaskStatusRequest {
  status: string;
  by?: string;
}

// Execution types
export interface ExecutionState {
  plan_id: string;
  branch: string;
  tasks: string[];
  task_status_map: Record<string, string>;
}

export interface RunningTask {
  task_id: string;
  task_name: string;
  plan_id: string;
  branch: string;
  agent?: AgentLease;
  started_at?: string;
  heartbeat_at?: string;
}

// Config types
export interface Config {
  server_host: string;
  server_port: number;
  max_parallel: number;
  default_timeout_seconds: number;
  log_level: string;
  yolo_mode: boolean;
}

export interface AgentRegistration {
  name: string;
  type: string;
  spawn_command: string;
  model?: string;
  tool_permissions?: string[];
  timeout_seconds?: number;
}

export interface AgentsResponse {
  agents: AgentRegistration[];
}

// List response wrapper
export interface ListResponse<T> {
  items: T[];
  total: number;
}

// Error response
export interface ApiErrorResponse {
  error: string;
  message: string;
  status: number;
}
