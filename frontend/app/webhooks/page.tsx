'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Bell, Plus, Trash2, TestTube, ExternalLink, CheckCircle, XCircle, Clock } from 'lucide-react';
import { toast } from 'sonner';

interface Webhook {
  id: string;
  name: string;
  url: string;
  type: 'slack' | 'discord' | 'custom';
  config: {
    secret?: string;
    channel?: string;
    username?: string;
    icon_url?: string;
  };
  severityFilter: 'all' | 'critical' | 'high' | 'medium' | 'low';
  enabled: boolean;
  successCount: number;
  failureCount: number;
  lastError?: string;
  createdAt: string;
  updatedAt: string;
}

interface NotificationStats {
  totalWebhooks: number;
  activeWebhooks: number;
  totalSent: number;
  totalFailed: number;
  recentActivity: Array<{
    id: string;
    webhookId: string;
    status: string;
    createdAt: string;
    sentAt?: string;
  }>;
}

export default function WebhooksPage() {
  const [webhooks, setWebhooks] = useState<Webhook[]>([]);
  const [stats, setStats] = useState<NotificationStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [formData, setFormData] = useState({
    name: '',
    url: '',
    type: 'custom' as 'slack' | 'discord' | 'custom',
    severityFilter: 'all' as 'all' | 'critical' | 'high' | 'medium' | 'low',
    config: {
      channel: '',
      username: '',
      icon_url: '',
    },
  });

  useEffect(() => {
    fetchWebhooks();
    fetchStats();
  }, []);

  const fetchWebhooks = async () => {
    try {
      const response = await fetch('/api/webhooks');
      if (response.ok) {
        const data = await response.json();
        setWebhooks(data);
      }
    } catch (error) {
      toast.error('Failed to fetch webhooks');
    } finally {
      setIsLoading(false);
    }
  };

  const fetchStats = async () => {
    try {
      const response = await fetch('/api/webhooks/stats/overview');
      if (response.ok) {
        const data = await response.json();
        setStats(data);
      }
    } catch (error) {
      console.error('Failed to fetch stats:', error);
    }
  };

  const handleCreateWebhook = async (e: React.FormEvent) => {
    e.preventDefault();
    
    try {
      const response = await fetch('/api/webhooks', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(formData),
      });

      if (response.ok) {
        toast.success('Webhook created successfully');
        setShowCreateForm(false);
        setFormData({
          name: '',
          url: '',
          type: 'custom',
          severityFilter: 'all',
          config: { channel: '', username: '', icon_url: '' },
        });
        fetchWebhooks();
        fetchStats();
      } else {
        toast.error('Failed to create webhook');
      }
    } catch (error) {
      toast.error('Failed to create webhook');
    }
  };

  const handleTestWebhook = async (webhookId: string) => {
    try {
      const response = await fetch(`/api/webhooks/${webhookId}/test`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ message: 'Test notification from Soroban Security Scanner' }),
      });

      if (response.ok) {
        toast.success('Test webhook sent successfully');
      } else {
        toast.error('Failed to send test webhook');
      }
    } catch (error) {
      toast.error('Failed to send test webhook');
    }
  };

  const handleDeleteWebhook = async (webhookId: string) => {
    if (!confirm('Are you sure you want to delete this webhook?')) {
      return;
    }

    try {
      const response = await fetch(`/api/webhooks/${webhookId}`, {
        method: 'DELETE',
      });

      if (response.ok) {
        toast.success('Webhook deleted successfully');
        fetchWebhooks();
        fetchStats();
      } else {
        toast.error('Failed to delete webhook');
      }
    } catch (error) {
      toast.error('Failed to delete webhook');
    }
  };

  const handleToggleWebhook = async (webhookId: string, enabled: boolean) => {
    try {
      const response = await fetch(`/api/webhooks/${webhookId}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ enabled }),
      });

      if (response.ok) {
        toast.success(`Webhook ${enabled ? 'enabled' : 'disabled'} successfully`);
        fetchWebhooks();
        fetchStats();
      } else {
        toast.error('Failed to update webhook');
      }
    } catch (error) {
      toast.error('Failed to update webhook');
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-gray-900"></div>
      </div>
    );
  }

  return (
    <div className="container mx-auto py-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold flex items-center gap-2">
            <Bell className="h-8 w-8" />
            Notification Settings
          </h1>
          <p className="text-gray-600 mt-2">
            Configure webhook notifications for security alerts
          </p>
        </div>
        <Button
          onClick={() => setShowCreateForm(!showCreateForm)}
          className="flex items-center gap-2"
        >
          <Plus className="h-4 w-4" />
          Add Webhook
        </Button>
      </div>

      <Tabs defaultValue="webhooks" className="space-y-6">
        <TabsList>
          <TabsTrigger value="webhooks">Webhooks</TabsTrigger>
          <TabsTrigger value="stats">Statistics</TabsTrigger>
        </TabsList>

        <TabsContent value="webhooks" className="space-y-6">
          {showCreateForm && (
            <Card>
              <CardHeader>
                <CardTitle>Create New Webhook</CardTitle>
                <CardDescription>
                  Add a new webhook endpoint for security notifications
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleCreateWebhook} className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                      <Label htmlFor="name">Webhook Name</Label>
                      <Input
                        id="name"
                        value={formData.name}
                        onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                        placeholder="My Security Webhook"
                        required
                      />
                    </div>
                    <div>
                      <Label htmlFor="type">Webhook Type</Label>
                      <Select
                        value={formData.type}
                        onValueChange={(value: 'slack' | 'discord' | 'custom') =>
                          setFormData({ ...formData, type: value })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="slack">Slack</SelectItem>
                          <SelectItem value="discord">Discord</SelectItem>
                          <SelectItem value="custom">Custom</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>

                  <div>
                    <Label htmlFor="url">Webhook URL</Label>
                    <Input
                      id="url"
                      type="url"
                      value={formData.url}
                      onChange={(e) => setFormData({ ...formData, url: e.target.value })}
                      placeholder="https://hooks.slack.com/services/..."
                      required
                    />
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                      <Label htmlFor="severity">Severity Filter</Label>
                      <Select
                        value={formData.severityFilter}
                        onValueChange={(value: 'all' | 'critical' | 'high' | 'medium' | 'low') =>
                          setFormData({ ...formData, severityFilter: value })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="all">All Vulnerabilities</SelectItem>
                          <SelectItem value="critical">Critical Only</SelectItem>
                          <SelectItem value="high">High & Critical</SelectItem>
                          <SelectItem value="medium">Medium & Above</SelectItem>
                          <SelectItem value="low">All Severities</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>

                    {formData.type === 'slack' && (
                      <div>
                        <Label htmlFor="channel">Slack Channel (Optional)</Label>
                        <Input
                          id="channel"
                          value={formData.config.channel}
                          onChange={(e) =>
                            setFormData({
                              ...formData,
                              config: { ...formData.config, channel: e.target.value },
                            })
                          }
                          placeholder="#security-alerts"
                        />
                      </div>
                    )}
                  </div>

                  {formData.type === 'slack' && (
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div>
                        <Label htmlFor="username">Bot Username (Optional)</Label>
                        <Input
                          id="username"
                          value={formData.config.username}
                          onChange={(e) =>
                            setFormData({
                              ...formData,
                              config: { ...formData.config, username: e.target.value },
                            })
                          }
                          placeholder="Security Scanner"
                        />
                      </div>
                      <div>
                        <Label htmlFor="icon">Icon URL (Optional)</Label>
                        <Input
                          id="icon"
                          type="url"
                          value={formData.config.icon_url}
                          onChange={(e) =>
                            setFormData({
                              ...formData,
                              config: { ...formData.config, icon_url: e.target.value },
                            })
                          }
                          placeholder="https://example.com/icon.png"
                        />
                      </div>
                    </div>
                  )}

                  <div className="flex gap-2">
                    <Button type="submit">Create Webhook</Button>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => setShowCreateForm(false)}
                    >
                      Cancel
                    </Button>
                  </div>
                </form>
              </CardContent>
            </Card>
          )}

          <div className="grid gap-4">
            {webhooks.map((webhook) => (
              <Card key={webhook.id}>
                <CardContent className="p-6">
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-2">
                        <h3 className="font-semibold text-lg">{webhook.name}</h3>
                        <Badge variant={webhook.enabled ? 'default' : 'secondary'}>
                          {webhook.enabled ? 'Active' : 'Inactive'}
                        </Badge>
                        <Badge variant="outline">{webhook.type}</Badge>
                      </div>
                      
                      <p className="text-sm text-gray-600 mb-2">{webhook.url}</p>
                      
                      <div className="flex items-center gap-4 text-sm">
                        <span className="flex items-center gap-1">
                          <Badge variant="outline">{webhook.severityFilter}</Badge>
                        </span>
                        <span className="flex items-center gap-1 text-green-600">
                          <CheckCircle className="h-4 w-4" />
                          {webhook.successCount} sent
                        </span>
                        <span className="flex items-center gap-1 text-red-600">
                          <XCircle className="h-4 w-4" />
                          {webhook.failureCount} failed
                        </span>
                      </div>

                      {webhook.lastError && (
                        <p className="text-sm text-red-600 mt-2">
                          Last error: {webhook.lastError}
                        </p>
                      )}
                    </div>

                    <div className="flex items-center gap-2">
                      <Switch
                        checked={webhook.enabled}
                        onCheckedChange={(enabled) => handleToggleWebhook(webhook.id, enabled)}
                      />
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleTestWebhook(webhook.id)}
                        className="flex items-center gap-1"
                      >
                        <TestTube className="h-4 w-4" />
                        Test
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleDeleteWebhook(webhook.id)}
                        className="text-red-600 hover:text-red-700"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}

            {webhooks.length === 0 && (
              <Card>
                <CardContent className="p-12 text-center">
                  <Bell className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                  <h3 className="text-lg font-semibold mb-2">No webhooks configured</h3>
                  <p className="text-gray-600 mb-4">
                    Get started by adding your first webhook to receive security notifications
                  </p>
                  <Button onClick={() => setShowCreateForm(true)}>
                    <Plus className="h-4 w-4 mr-2" />
                    Add Your First Webhook
                  </Button>
                </CardContent>
              </Card>
            )}
          </div>
        </TabsContent>

        <TabsContent value="stats" className="space-y-6">
          {stats && (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              <Card>
                <CardContent className="p-6">
                  <div className="flex items-center gap-2">
                    <Bell className="h-8 w-8 text-blue-600" />
                    <div>
                      <p className="text-2xl font-bold">{stats.totalWebhooks}</p>
                      <p className="text-gray-600">Total Webhooks</p>
                    </div>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardContent className="p-6">
                  <div className="flex items-center gap-2">
                    <CheckCircle className="h-8 w-8 text-green-600" />
                    <div>
                      <p className="text-2xl font-bold">{stats.activeWebhooks}</p>
                      <p className="text-gray-600">Active Webhooks</p>
                    </div>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardContent className="p-6">
                  <div className="flex items-center gap-2">
                    <CheckCircle className="h-8 w-8 text-green-600" />
                    <div>
                      <p className="text-2xl font-bold">{stats.totalSent}</p>
                      <p className="text-gray-600">Notifications Sent</p>
                    </div>
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardContent className="p-6">
                  <div className="flex items-center gap-2">
                    <XCircle className="h-8 w-8 text-red-600" />
                    <div>
                      <p className="text-2xl font-bold">{stats.totalFailed}</p>
                      <p className="text-gray-600">Failed Notifications</p>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          <Card>
            <CardHeader>
              <CardTitle>Recent Activity</CardTitle>
              <CardDescription>Latest webhook notification attempts</CardDescription>
            </CardHeader>
            <CardContent>
              {stats?.recentActivity && stats.recentActivity.length > 0 ? (
                <div className="space-y-2">
                  {stats.recentActivity.map((activity) => (
                    <div
                      key={activity.id}
                      className="flex items-center justify-between p-3 border rounded-lg"
                    >
                      <div className="flex items-center gap-2">
                        {activity.status === 'sent' ? (
                          <CheckCircle className="h-4 w-4 text-green-600" />
                        ) : activity.status === 'failed' ? (
                          <XCircle className="h-4 w-4 text-red-600" />
                        ) : (
                          <Clock className="h-4 w-4 text-yellow-600" />
                        )}
                        <span className="font-medium">{activity.status}</span>
                        <span className="text-sm text-gray-600">
                          Webhook ID: {activity.webhookId.slice(0, 8)}...
                        </span>
                      </div>
                      <div className="text-sm text-gray-600">
                        {activity.sentAt ? `Sent: ${new Date(activity.sentAt).toLocaleString()}` : `Created: ${new Date(activity.createdAt).toLocaleString()}`}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-gray-600 text-center py-4">No recent activity</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
