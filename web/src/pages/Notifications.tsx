import React, { useEffect, useState } from 'react';
import {
  Card,
  Collapse,
  Form,
  Input,
  Select,
  Switch,
  Button,
  Space,
  Message,
  Divider,
  Typography,
  InputNumber,
  Spin,
  Tag,
} from '@arco-design/web-react';
import { IconSend, IconSave, IconPlus, IconDelete } from '@arco-design/web-react/icon';
import { notificationApi } from '@/api/notification';
import type {
  NotificationConfig,
  ChannelConfig,
  TelegramConfig,
  PushPlusConfig,
  SmtpConfig,
  ResendConfig,
  WeComConfig,
  WebhookConfig,
} from '@/types';

const { Text } = Typography;
const FormItem = Form.Item;
const CollapseItem = Collapse.Item;

// ─── 响应式钩子 ───────────────────────────────────────────────────────────────

const useMobile = () => {
  const [mobile, setMobile] = useState(() => window.innerWidth < 640);
  useEffect(() => {
    const handler = () => setMobile(window.innerWidth < 640);
    window.addEventListener('resize', handler);
    return () => window.removeEventListener('resize', handler);
  }, []);
  return mobile;
};

// ─── 默认空配置 ───────────────────────────────────────────────────────────────

const defaultTelegram = (): TelegramConfig => ({ bot_token: '', chat_id: '', proxy: '' });
const defaultPushPlus = (): PushPlusConfig => ({ token: '', topic: '' });
const defaultSmtp     = (): SmtpConfig     => ({ host: '', port: 465, username: '', password: '', from: '', to: [], use_tls: true });
const defaultResend   = (): ResendConfig   => ({ api_key: '', from: '', to: [] });
const defaultWeCom    = (): WeComConfig    => ({ webhook_url: '' });
const defaultWebhook   = (): WebhookConfig => ({ url: '', method: 'POST', headers: {}, body_template: '' });

const getCh = <T,>(channels: ChannelConfig[], type: string, def: () => T): T => {
  const ch = channels.find((c) => c.type === type);
  return ch ? (ch.config as T) : def();
};
const getChEnabled = (channels: ChannelConfig[], type: string) =>
  channels.find((c) => c.type === type)?.enabled ?? false;

// ─── 状态标签 ─────────────────────────────────────────────────────────────────

const StatusTag: React.FC<{ enabled: boolean }> = ({ enabled }) =>
  enabled
    ? <Tag color="green" size="small">已启用</Tag>
    : <Tag color="gray"  size="small">未启用</Tag>;

// ─── 渠道折叠面板 Header（移动端自动换行）────────────────────────────────────

interface ChannelHeaderProps {
  label: string;
  enabled: boolean;
  testing: boolean;
  onToggle: (v: boolean) => void;
  onTest: () => void;
}

const ChannelHeader: React.FC<ChannelHeaderProps> = ({ label, enabled, testing, onToggle, onTest }) => (
  <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '6px 12px', minWidth: 0 }}>
    {/* 左侧：名称 + 状态 */}
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0, flex: '1 1 auto' }}>
      <span style={{ fontWeight: 500, whiteSpace: 'nowrap' }}>{label}</span>
      <StatusTag enabled={enabled} />
    </div>
    {/* 右侧：开关 + 测试（点击阻止冒泡，防止触发折叠） */}
    <span
      style={{ display: 'flex', alignItems: 'center', gap: 8, flex: '0 0 auto' }}
      onClick={(e) => e.stopPropagation()}
    >
      <Switch size="small" checked={enabled} onChange={onToggle} />
      <Button size="mini" type="outline" icon={<IconSend />} loading={testing} onClick={onTest}>
        测试
      </Button>
    </span>
  </div>
);

// ─── 代码块 ───────────────────────────────────────────────────────────────────

const CodeBlock: React.FC<{ code: string }> = ({ code }) => (
  <pre style={{
    background: 'var(--color-fill-2)',
    padding: '8px 12px',
    borderRadius: 4,
    fontSize: 12,
    margin: 0,
    overflowX: 'auto',   // 移动端横向滚动
    whiteSpace: 'pre',
  }}>
    {code}
  </pre>
);

// ─── 主页面 ───────────────────────────────────────────────────────────────────

const Notifications: React.FC = () => {
  const isMobile = useMobile();

  const [loading,  setLoading]  = useState(false);
  const [saving,   setSaving]   = useState(false);
  const [testing,  setTesting]  = useState<string | null>(null);

  // 全局开关
  const [enabled,   setEnabled]   = useState(false);
  const [onSuccess, setOnSuccess] = useState(false);
  const [onFailure, setOnFailure] = useState(true);
  const [onKilled,  setOnKilled]  = useState(true);

  // 渠道启用开关
  const [tgOn,   setTgOn]   = useState(false);
  const [ppOn,   setPpOn]   = useState(false);
  const [smtpOn, setSmtpOn] = useState(false);
  const [resOn,  setResOn]  = useState(false);
  const [wcOn,   setWcOn]   = useState(false);
  const [whOn,    setWhOn]    = useState(false);

  // 渠道配置
  const [tg,    setTg]    = useState<TelegramConfig>(defaultTelegram());
  const [pp,    setPp]    = useState<PushPlusConfig>(defaultPushPlus());
  const [smtp,  setSmtp]  = useState<SmtpConfig>(defaultSmtp());
  const [res,   setRes]   = useState<ResendConfig>(defaultResend());
  const [wecom, setWecom] = useState<WeComConfig>(defaultWeCom());
  const [wh,    setWh]    = useState<WebhookConfig>(defaultWebhook());
  const [whHeaders, setWhHeaders] = useState<Array<{id: number; key: string; value: string}>>([]);

  // 多收件人：用逗号分隔字符串维护，保存时 split
  const [smtpTo, setSmtpTo] = useState('');
  const [resTo,  setResTo]  = useState('');

  useEffect(() => { loadConfig(); }, []);

  const loadConfig = async () => {
    setLoading(true);
    try {
      const cfg: NotificationConfig = await notificationApi.getConfig();
      setEnabled(cfg.enabled);
      setOnSuccess(cfg.on_success);
      setOnFailure(cfg.on_failure);
      setOnKilled(cfg.on_killed);

      setTgOn(getChEnabled(cfg.channels, 'telegram'));
      setPpOn(getChEnabled(cfg.channels, 'pushplus'));
      setSmtpOn(getChEnabled(cfg.channels, 'smtp'));
      setResOn(getChEnabled(cfg.channels, 'resend'));
      setWcOn(getChEnabled(cfg.channels, 'wecom'));

      const tgCfg   = getCh<TelegramConfig>(cfg.channels, 'telegram', defaultTelegram);
      const ppCfg   = getCh<PushPlusConfig>(cfg.channels, 'pushplus', defaultPushPlus);
      const smtpCfg = getCh<SmtpConfig>    (cfg.channels, 'smtp',     defaultSmtp);
      const resCfg  = getCh<ResendConfig>  (cfg.channels, 'resend',   defaultResend);
      const wcCfg   = getCh<WeComConfig>   (cfg.channels, 'wecom',    defaultWeCom);

      setTg(tgCfg);
      setPp(ppCfg);
      setSmtp(smtpCfg);
      setRes(resCfg);
      setWecom(wcCfg);
      setSmtpTo(smtpCfg.to.join(', '));
      setResTo(resCfg.to.join(', '));

      const whCh = cfg.channels.find(c => c.type === 'webhook');
      if (whCh) {
        setWhOn(whCh.enabled);
        const whCfg = whCh.config as WebhookConfig;
        setWh({ url: whCfg.url || '', method: whCfg.method || 'POST', headers: {}, body_template: whCfg.body_template || '' });
        setWhHeaders(Object.entries(whCfg.headers || {}).map(([k, v], i) => ({ id: i, key: k, value: v })));
      }
    } catch {
      Message.error('加载通知配置失败');
    } finally {
      setLoading(false);
    }
  };

  const toList = (s: string) => s.split(',').map((v) => v.trim()).filter(Boolean);

  const buildConfig = (): NotificationConfig => ({
    enabled,
    on_success: onSuccess,
    on_failure: onFailure,
    on_killed:  onKilled,
    channels: [
      { type: 'telegram', enabled: tgOn,   config: tg },
      { type: 'pushplus', enabled: ppOn,   config: pp },
      { type: 'smtp',     enabled: smtpOn, config: { ...smtp, to: toList(smtpTo) } },
      { type: 'resend',   enabled: resOn,  config: { ...res,  to: toList(resTo)  } },
      { type: 'wecom',    enabled: wcOn,   config: wecom },
      { type: 'webhook', enabled: whOn, config: { ...wh, headers: Object.fromEntries(whHeaders.filter(h => h.key).map(h => [h.key, h.value])) } as unknown as Record<string, unknown> },
    ],
  });

  const handleSave = async () => {
    setSaving(true);
    try {
      await notificationApi.updateConfig(buildConfig());
      Message.success('保存成功');
    } catch (e: any) {
      Message.error(e.response?.data || '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async (type: string, config: Record<string, unknown>) => {
    setTesting(type);
    try {
      await notificationApi.testChannel({ channel_type: type, config });
      Message.success('测试发送成功，请确认是否收到消息');
    } catch (e: any) {
      Message.error(`测试失败：${e.response?.data || e.message}`);
    } finally {
      setTesting(null);
    }
  };

  // 表单布局：移动端竖排，桌面端横排
  const formLayout  = isMobile ? 'vertical' : 'horizontal';
  const labelCol    = isMobile ? undefined : { span: 6 };
  const wrapperCol  = isMobile ? undefined : { span: 18 };

  return (
    <Spin loading={loading} style={{ width: '100%' }}>
      <div style={{ maxWidth: 820, margin: '0 auto', padding: isMobile ? '0 8px 32px' : '0 16px 32px' }}>

        {/* ── 全局设置 ─────────────────────────────────────────────── */}
        <Card style={{ marginBottom: 16 }} title="全局设置">
          <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
            <FormItem label="启用通知">
              <Space>
                <Switch checked={enabled} onChange={setEnabled} />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  总开关，关闭后所有渠道均不发送
                </Text>
              </Space>
            </FormItem>

            <FormItem label="触发条件" style={{ marginBottom: 0 }}>
              {/* wrap 属性确保移动端自动换行 */}
              <Space wrap size={[16, 8]}>
                <Space size={6}>
                  <Switch size="small" checked={onFailure} onChange={setOnFailure} />
                  <Text>任务失败</Text>
                </Space>
                <Space size={6}>
                  <Switch size="small" checked={onKilled} onChange={setOnKilled} />
                  <Text>任务终止</Text>
                </Space>
                <Space size={6}>
                  <Switch size="small" checked={onSuccess} onChange={setOnSuccess} />
                  <Text>任务成功</Text>
                </Space>
              </Space>
            </FormItem>
          </Form>
        </Card>

        {/* ── 渠道配置（折叠面板）────────────────────────────────────── */}
        <Card style={{ marginBottom: 16 }} title="通知渠道" bodyStyle={{ padding: 0 }}>
          <Collapse bordered={false}>

            {/* ── Telegram ── */}
            <CollapseItem
              name="telegram"
              header={
                <ChannelHeader
                  label="Telegram"
                  enabled={tgOn}
                  testing={testing === 'telegram'}
                  onToggle={setTgOn}
                  onTest={() => handleTest('telegram', tg as unknown as Record<string, unknown>)}
                />
              }
            >
              <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
                <FormItem label="Bot Token" required>
                  <Input
                    placeholder="110201543:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw"
                    value={tg.bot_token}
                    onChange={(v) => setTg({ ...tg, bot_token: v })}
                  />
                </FormItem>
                <FormItem label="Chat ID" required>
                  <Input
                    placeholder="-1001234567890 或个人数字 ID"
                    value={tg.chat_id}
                    onChange={(v) => setTg({ ...tg, chat_id: v })}
                  />
                </FormItem>
                <FormItem label="代理地址" style={{ marginBottom: 0 }}>
                  <Input
                    placeholder="http://127.0.0.1:7890（可选）"
                    value={tg.proxy || ''}
                    onChange={(v) => setTg({ ...tg, proxy: v })}
                  />
                </FormItem>
              </Form>
            </CollapseItem>

            {/* ── 企业微信机器人 ── */}
            <CollapseItem
              name="wecom"
              header={
                <ChannelHeader
                  label="企业微信机器人"
                  enabled={wcOn}
                  testing={testing === 'wecom'}
                  onToggle={setWcOn}
                  onTest={() => handleTest('wecom', wecom as unknown as Record<string, unknown>)}
                />
              }
            >
              <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
                <FormItem label="Webhook URL" required style={{ marginBottom: 0 }}>
                  <Input
                    placeholder="https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx"
                    value={wecom.webhook_url}
                    onChange={(v) => setWecom({ webhook_url: v })}
                  />
                </FormItem>
              </Form>
            </CollapseItem>

            {/* ── Webhook ── */}
            <CollapseItem
              name="webhook"
              header={
                <ChannelHeader
                  label="自定义 Webhook"
                  enabled={whOn}
                  testing={testing === 'webhook'}
                  onToggle={setWhOn}
                  onTest={() => handleTest('webhook', { ...wh, headers: (() => { const m: Record<string,string> = {}; whHeaders.forEach(h => { if(h.key) m[h.key]=h.value; }); return m; })() } as unknown as Record<string, unknown>)}
                />
              }
            >
              <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
                <FormItem label="请求方式">
                  <Select
                    value={wh.method}
                    onChange={(v) => setWh({ ...wh, method: v })}
                    options={['GET','POST','PUT','PATCH'].map(m => ({ label: m, value: m }))}
                    style={{ width: isMobile ? '100%' : 160 }}
                  />
                </FormItem>
                <FormItem label="URL" required>
                  <Input
                    placeholder="https://example.com/webhook"
                    value={wh.url}
                    onChange={(v) => setWh({ ...wh, url: v })}
                  />
                </FormItem>
                <FormItem label="自定义 Headers">
                  <Space direction="vertical" style={{ width: '100%' }} size={6}>
                    {whHeaders.map((row) => (
                      <Space key={row.id} style={{ width: '100%', flexWrap: 'nowrap' as const }} size={6}>
                        <Input
                          placeholder="Header 名"
                          value={row.key}
                          style={{ flex: 1, minWidth: 0 }}
                          onChange={(v) => setWhHeaders(prev => prev.map(h => h.id === row.id ? { ...h, key: v } : h))}
                        />
                        <Input
                          placeholder="值"
                          value={row.value}
                          style={{ flex: 1, minWidth: 0 }}
                          onChange={(v) => setWhHeaders(prev => prev.map(h => h.id === row.id ? { ...h, value: v } : h))}
                        />
                        <Button
                          icon={<IconDelete />}
                          size="mini"
                          status="danger"
                          type="text"
                          onClick={() => setWhHeaders(prev => prev.filter(h => h.id !== row.id))}
                        />
                      </Space>
                    ))}
                    <Button
                      icon={<IconPlus />}
                      size="small"
                      type="dashed"
                      onClick={() => setWhHeaders(prev => [...prev, { id: Date.now(), key: '', value: '' }])}
                    >
                      添加 Header
                    </Button>
                  </Space>
                </FormItem>
                {wh.method !== 'GET' && (
                  <FormItem label="Body 模板" style={{ marginBottom: 0 }}>
                    <Input.TextArea
                      placeholder={'留空则发送默认 JSON：\n{"title":"{title}","content":"{content}"}\n\n可用变量：{title}、{content}'}
                      value={wh.body_template}
                      onChange={(v) => setWh({ ...wh, body_template: v })}
                      autoSize={{ minRows: 3, maxRows: 8 }}
                      style={{ fontFamily: 'monospace', fontSize: 12 }}
                    />
                  </FormItem>
                )}
              </Form>
            </CollapseItem>

            {/* ── PushPlus 微信 ── */}
            <CollapseItem
              name="pushplus"
              header={
                <ChannelHeader
                  label="PushPlus（微信）"
                  enabled={ppOn}
                  testing={testing === 'pushplus'}
                  onToggle={setPpOn}
                  onTest={() => handleTest('pushplus', pp as unknown as Record<string, unknown>)}
                />
              }
            >
              <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
                <FormItem label="Token" required>
                  <Input
                    placeholder="前往 pushplus.plus 获取 Token"
                    value={pp.token}
                    onChange={(v) => setPp({ ...pp, token: v })}
                  />
                </FormItem>
                <FormItem label="群组编码" style={{ marginBottom: 0 }}>
                  <Input
                    placeholder="可选，群组推送时填写"
                    value={pp.topic || ''}
                    onChange={(v) => setPp({ ...pp, topic: v })}
                  />
                </FormItem>
              </Form>
            </CollapseItem>

            {/* ── SMTP 邮件 ── */}
            <CollapseItem
              name="smtp"
              header={
                <ChannelHeader
                  label="SMTP 邮件"
                  enabled={smtpOn}
                  testing={testing === 'smtp'}
                  onToggle={setSmtpOn}
                  onTest={() => handleTest('smtp', { ...smtp, to: toList(smtpTo) } as unknown as Record<string, unknown>)}
                />
              }
            >
              <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
                {/* 移动端竖排；桌面端 host 占 2/3、port 占 1/3 */}
                {isMobile ? (
                  <>
                    <FormItem label="SMTP 服务器" required>
                      <Input
                        placeholder="smtp.qq.com"
                        value={smtp.host}
                        onChange={(v) => setSmtp({ ...smtp, host: v })}
                      />
                    </FormItem>
                    <FormItem label="端口">
                      <InputNumber
                        min={1} max={65535}
                        value={smtp.port}
                        onChange={(v) => setSmtp({ ...smtp, port: v || 465 })}
                        style={{ width: '100%' }}
                      />
                    </FormItem>
                  </>
                ) : (
                  <FormItem label="SMTP 服务器" required>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <Input
                        placeholder="smtp.qq.com"
                        value={smtp.host}
                        onChange={(v) => setSmtp({ ...smtp, host: v })}
                        style={{ flex: 1 }}
                      />
                      <InputNumber
                        min={1} max={65535}
                        value={smtp.port}
                        onChange={(v) => setSmtp({ ...smtp, port: v || 465 })}
                        style={{ width: 90 }}
                      />
                    </div>
                  </FormItem>
                )}
                <FormItem label="用户名" required>
                  <Input
                    placeholder="your@email.com"
                    value={smtp.username}
                    onChange={(v) => setSmtp({ ...smtp, username: v })}
                  />
                </FormItem>
                <FormItem label="密码 / 授权码" required>
                  <Input.Password
                    placeholder="邮箱密码或授权码"
                    value={smtp.password}
                    onChange={(v) => setSmtp({ ...smtp, password: v })}
                  />
                </FormItem>
                <FormItem label="发件人" required>
                  <Input
                    placeholder="your@email.com 或 朱雀 <your@email.com>"
                    value={smtp.from}
                    onChange={(v) => setSmtp({ ...smtp, from: v })}
                  />
                </FormItem>
                <FormItem label="收件人" required>
                  <Input
                    placeholder="多个地址用英文逗号分隔"
                    value={smtpTo}
                    onChange={setSmtpTo}
                  />
                </FormItem>
                <FormItem label="SSL/TLS" style={{ marginBottom: 0 }}>
                  <Space size={8}>
                    <Switch
                      checked={smtp.use_tls}
                      onChange={(v) => setSmtp({ ...smtp, use_tls: v })}
                    />
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      端口 465 开启；587 STARTTLS 关闭
                    </Text>
                  </Space>
                </FormItem>
              </Form>
            </CollapseItem>

            {/* ── Resend ── */}
            <CollapseItem
              name="resend"
              header={
                <ChannelHeader
                  label="Resend"
                  enabled={resOn}
                  testing={testing === 'resend'}
                  onToggle={setResOn}
                  onTest={() => handleTest('resend', { ...res, to: toList(resTo) } as unknown as Record<string, unknown>)}
                />
              }
            >
              <Form layout={formLayout} labelCol={labelCol} wrapperCol={wrapperCol}>
                <FormItem label="API Key" required>
                  <Input.Password
                    placeholder="re_xxxxxxxxxxxx"
                    value={res.api_key}
                    onChange={(v) => setRes({ ...res, api_key: v })}
                  />
                </FormItem>
                <FormItem label="发件人" required>
                  <Input
                    placeholder="朱雀通知 <notify@yourdomain.com>"
                    value={res.from}
                    onChange={(v) => setRes({ ...res, from: v })}
                  />
                </FormItem>
                <FormItem label="收件人" required style={{ marginBottom: 0 }}>
                  <Input
                    placeholder="多个地址用英文逗号分隔"
                    value={resTo}
                    onChange={setResTo}
                  />
                </FormItem>
              </Form>
            </CollapseItem>

          </Collapse>
        </Card>

        {/* ── 脚本调用说明 ─────────────────────────────────────────── */}
        <Card style={{ marginBottom: 16 }} title="脚本内发送通知">
          <Text type="secondary" style={{ display: 'block', marginBottom: 12, fontSize: 13 }}>
            执行器启动任务时自动注入以下工具，无需额外配置。
          </Text>
          <Divider orientation="left" style={{ fontSize: 12, color: '#999', margin: '8px 0' }}>Shell</Divider>
          <CodeBlock code={`notify "签到结果" "账号A: +50豆\\n账号B: 失败"`} />
          <Divider orientation="left" style={{ fontSize: 12, color: '#999', margin: '12px 0 8px' }}>Python</Divider>
          <CodeBlock code={`from notify import send\nsend("签到结果", "账号A: +50豆")`} />
          <Divider orientation="left" style={{ fontSize: 12, color: '#999', margin: '12px 0 8px' }}>Node.js</Divider>
          <CodeBlock code={`const { sendNotify } = require('./sendNotify')\nsendNotify('签到结果', '账号A: +50豆')`} />
        </Card>

        {/* ── 保存按钮 ─────────────────────────────────────────────── */}
        <div style={{ textAlign: 'right' }}>
          <Button
            type="primary"
            icon={<IconSave />}
            loading={saving}
            onClick={handleSave}
            size="large"
            long={isMobile}   // 移动端占满宽度
          >
            保存配置
          </Button>
        </div>

      </div>
    </Spin>
  );
};

export default Notifications;
