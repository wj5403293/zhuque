import request from '@/utils/request';
import type { Task, Execution } from '@/types';

export const taskApi = {
  // 获取任务列表
  list: (params?: { search?: string }) =>
    request.get<Task[]>('/tasks', { params }),

  // 获取任务列表（简化版，仅id、name和enabled）
  listSimple: () => request.get<Array<{ id: number; name: string; enabled: boolean }>>('/tasks?fields=simple'),

  // 获取任务详情
  get: (id: number) => request.get<Task>(`/tasks/${id}`),

  // 创建任务
  create: (data: Partial<Task>) => request.post('/tasks', data),

  // 更新任务
  update: (id: number, data: Partial<Task>) => request.put(`/tasks/${id}`, data),

  // 删除任务
  delete: (id: number) => request.delete(`/tasks/${id}`),

  // 立即执行任务
  run: (id: number) => request.post(`/tasks/${id}/run`),

  // 终止任务
  kill: (id: number) => request.delete(`/tasks/${id}/kill`),

  // 启用/禁用任务
  toggleEnabled: (id: number, enabled: boolean) =>
    request.put(`/tasks/${id}`, { enabled }),

  // 获取运行中的任务
  listRunning: () => request.get<Task[]>('/tasks/running'),

  // 获取执行记录
  listExecutions: () => request.get<Execution[]>('/executions'),
};
