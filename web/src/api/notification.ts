import request from '@/utils/request';
import type { NotificationConfig, TestChannelRequest } from '@/types';

export const notificationApi = {
  getConfig: (): Promise<NotificationConfig> =>
    request.get('/notification/config'),

  updateConfig: (config: NotificationConfig): Promise<NotificationConfig> =>
    request.post('/notification/config', config),

  testChannel: (req: TestChannelRequest): Promise<{ success: boolean; message: string }> =>
    request.post('/notification/test', req),
};
