import type {
  Plan, TaskRef, Task, TaskStatus, AgentLease, StatusTransition,
  ExecutionState, RunningTask, Config, AgentRegistration,
  AgentsResponse, ListResponse, ApiErrorResponse,
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
  body?: unknown,
  queryParams?: Record<string, string | undefined>,
): Promise<T> {
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

  return data as T;
}

// ===== Plan API =====

export async function listPlans(branch?: string, status?: string): Promise<ListResponse<Plan>> {
  return request('GET', '/plans', undefined, { branch, status });
}

export async function getPlan(branch: string, planId: string): Promise<Plan> {
  return request('GET', `/plans/${branch}/${planId}`);
}

export async function createPlan(req: CreatePlanRequest): Promise<Plan> {
  return request('POST', '/plans', req);
}

export async function updatePlan(branch: string, planId: string, req: UpdatePlanRequest): Promise<Plan> {
  return request('PUT', `/plans/${branch}/${planId}`, req);
}

export async function deletePlan(branch: string, planId: string): Promise<void> {
  return request('DELETE', `/plans/${branch}/${planId}`);
}

export async function transitionPlanStatus(
  branch: string,
  planId: string,
  req: TransitionPlanStatusRequest,
): Promise<Plan> {
  return request('PATCH', `/plans/${branch}/${planId}/status`, req);
}

// ===== Task API =====

export async function listTasks(
  branch: string,
  planId: string,
  status?: string,
): Promise<ListResponse<Task>> {
  return request('GET', `/plans/${branch}/${planId}/tasks`, undefined, { status });
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
  return request('POST', `/plans/${branch}/${planId}/tasks`, req);
}

export async function updateTask(
  branch: string,
  planId: string,
  taskId: string,
  req: UpdateTaskRequest,
): Promise<Task> {
  return request('PUT', `/plans/${branch}/${planId}/tasks/${taskId}`, req);
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
  return request('PATCH', `/plans/${branch}/${planId}/tasks/${taskId}/status`, req);
}

// ===== Execution API =====

export async function getExecutionState(
  branch: string,
  planId: string,
): Promise<ExecutionState> {
  return request('GET', `/plans/${branch}/${planId}/execution`);
}

export async function listRunningTasks(): Promise<RunningTask[]> {
  return request('GET', '/running');
}

export async function healthCheck(): Promise<{ status: string }> {
  return request('GET', '/health');
}

// ===== Config API =====

export async function getConfig(): Promise<Config> {
  return request('GET', '/config');
}

export async function listAgents(): Promise<AgentsResponse> {
  return request('GET', '/agents');
}
