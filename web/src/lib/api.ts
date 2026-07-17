import type {
  Plan, TaskRef, Task, TaskStatus, AgentLease, StatusTransition,
  RunningTask, Config, PatchConfigRequest, AgentRegistration,
  AgentsResponse, ListResponse, ApiErrorResponse, LogEntry,
  CreatePlanRequest, UpdatePlanRequest, TransitionPlanStatusRequest,
  CreateTaskRequest, UpdateTaskRequest, TransitionTaskStatusRequest,
} from './types';

const API_PREFIX = '/api/v1';

/**
 * Base request helper — fetch wrapper with /api/v1 prefix, JSON serialization, error handling.
 */
export async function request<T>(
  method: string,
  path: string,
  options?: { body?: unknown; queryParams?: Record<string, string | undefined>; signal?: AbortSignal },
): Promise<T> {
  const { body, queryParams, signal } = options ?? {};

  let url = `${API_PREFIX}${path}`;

  // Append query parameters
  if (queryParams) {
    const searchParams = new URLSearchParams();
    for (const [key, value] of Object.entries(queryParams)) {
      if (value !== undefined) {
        searchParams.append(key, value);
      }
    }
    const qs = searchParams.toString();
    if (qs) {
      url += `?${qs}`;
    }
  }

  const response = await fetch(url, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
    signal,
  });

  if (!response.ok) {
    const contentType = response.headers.get('content-type') || '';
    let errorMessage: string;

    if (contentType.includes('application/json')) {
      const errorData = await response.json() as ApiErrorResponse;
      errorMessage = errorData?.message || `API error ${response.status}`;
    } else {
      const text = await response.text();
      errorMessage = text || `API error ${response.status}`;
    }

    throw new Error(errorMessage);
  }

  const data = await response.json();

  // NOTE: unchecked cast — server response shape assumed to match T
  return data as T;
}

// ===== Plan API =====

export async function listPlans(branch?: string, status?: string): Promise<ListResponse<Plan>> {
  return request('GET', '/plans', { queryParams: { branch, status } });
}

export async function getPlan(branch: string, planId: string): Promise<Plan> {
  return request('GET', `/plans/${branch}/${planId}`);
}

export async function createPlan(req: CreatePlanRequest): Promise<Plan> {
  return request('POST', '/plans', { body: req });
}

export async function updatePlan(branch: string, planId: string, req: UpdatePlanRequest): Promise<Plan> {
  return request('PATCH', `/plans/${branch}/${planId}`, { body: req });
}

export async function deletePlan(branch: string, planId: string): Promise<void> {
  return request('DELETE', `/plans/${branch}/${planId}`);
}

export async function transitionPlanStatus(
  branch: string,
  planId: string,
  req: TransitionPlanStatusRequest,
): Promise<Plan> {
  return request('PATCH', `/plans/${branch}/${planId}/status`, { body: req });
}

// ===== Task API =====

export async function listTasks(
  branch: string,
  planId: string,
  status?: string,
): Promise<ListResponse<Task>> {
  return request('GET', `/plans/${branch}/${planId}/tasks`, { queryParams: { status } });
}

export async function getTask(
  branch: string,
  planId: string,
  taskId: string,
): Promise<Task> {
  return request('GET', `/plans/${branch}/${planId}/tasks/${taskId}`);
}

export async function createTask(
  branch: string,
  planId: string,
  req: CreateTaskRequest,
): Promise<Task> {
  return request('POST', `/plans/${branch}/${planId}/tasks`, { body: req });
}

export async function updateTask(
  branch: string,
  planId: string,
  taskId: string,
  req: UpdateTaskRequest,
): Promise<Task> {
  return request('PATCH', `/plans/${branch}/${planId}/tasks/${taskId}`, { body: req });
}

export async function deleteTask(
  branch: string,
  planId: string,
  taskId: string,
): Promise<void> {
  return request('DELETE', `/plans/${branch}/${planId}/tasks/${taskId}`);
}

export async function transitionTaskStatus(
  branch: string,
  planId: string,
  taskId: string,
  req: TransitionTaskStatusRequest,
): Promise<Task> {
  return request('PATCH', `/plans/${branch}/${planId}/tasks/${taskId}/status`, { body: req });
}

// ===== Monitoring API =====

/**
 * Fetch execution logs for a specific task.
 * NOTE: Backend endpoint GET /plans/{branch}/{planId}/tasks/{taskId}/logs.
 */
export async function getTaskLogs(
  branch: string,
  planId: string,
  taskId: string,
  options?: { signal?: AbortSignal },
): Promise<LogEntry[]> {
  return await request('GET', `/plans/${branch}/${planId}/tasks/${taskId}/logs`, { signal: options?.signal });
}

// ===== Execution API =====

export async function listRunningTasks(options?: { signal?: AbortSignal }): Promise<RunningTask[]> {
  return request('GET', '/running', { signal: options?.signal });
}

export async function healthCheck(): Promise<{ status: string }> {
  return request('GET', '/health');
}

// ===== Config API =====

export async function getConfig(): Promise<Config> {
  return request('GET', '/config');
}

export async function listAgents(options?: { signal?: AbortSignal }): Promise<AgentsResponse> {
  return request('GET', '/agents', { signal: options?.signal });
}

export async function patchConfig(req: PatchConfigRequest): Promise<Config> {
  return request('PATCH', '/config', { body: req });
}
