import request from '@/utils/request';
import type { EnvVar } from '@/types';

export const envApi = {
  list: (params?: { search?: string }) =>
    request.get<EnvVar[]>('/env', { params }),
  get: (id: number) => request.get<EnvVar>(`/env/${id}`),
  create: (data: Partial<EnvVar>) => request.post('/env', data),
  update: (id: number, data: Partial<EnvVar>) => request.put(`/env/${id}`, data),
  delete: (id: number) => request.delete(`/env/${id}`),
};
