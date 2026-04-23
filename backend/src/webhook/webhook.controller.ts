import { Controller, Get, Post, Body, Param, Delete, Put, UseGuards, Request } from '@nestjs/common';
import { ApiTags, ApiOperation, ApiResponse, ApiBearerAuth } from '@nestjs/swagger';
import { JwtAuthGuard } from '../auth/jwt-auth.guard';
import { WebhookService } from './webhook.service';
import { CreateWebhookDto } from './dto/create-webhook.dto';
import { TestWebhookDto } from './dto/test-webhook.dto';

@ApiTags('webhooks')
@ApiBearerAuth()
@UseGuards(JwtAuthGuard)
@Controller('webhooks')
export class WebhookController {
  constructor(private readonly webhookService: WebhookService) {}

  @Post()
  @ApiOperation({ summary: 'Create a new webhook' })
  @ApiResponse({ status: 201, description: 'Webhook created successfully' })
  async createWebhook(@Body() createWebhookDto: CreateWebhookDto, @Request() req) {
    return await this.webhookService.createWebhook(createWebhookDto, req.user.userId);
  }

  @Get()
  @ApiOperation({ summary: 'Get all webhooks for the current user' })
  @ApiResponse({ status: 200, description: 'Webhooks retrieved successfully' })
  async getWebhooks(@Request() req) {
    return await this.webhookService.getWebhooks(req.user.userId);
  }

  @Get(':id')
  @ApiOperation({ summary: 'Get a specific webhook' })
  @ApiResponse({ status: 200, description: 'Webhook retrieved successfully' })
  async getWebhook(@Param('id') id: string, @Request() req) {
    return await this.webhookService.getWebhook(id, req.user.userId);
  }

  @Put(':id')
  @ApiOperation({ summary: 'Update a webhook' })
  @ApiResponse({ status: 200, description: 'Webhook updated successfully' })
  async updateWebhook(
    @Param('id') id: string,
    @Body() updateData: Partial<CreateWebhookDto>,
    @Request() req
  ) {
    return await this.webhookService.updateWebhook(id, req.user.userId, updateData);
  }

  @Delete(':id')
  @ApiOperation({ summary: 'Delete a webhook' })
  @ApiResponse({ status: 200, description: 'Webhook deleted successfully' })
  async deleteWebhook(@Param('id') id: string, @Request() req) {
    await this.webhookService.deleteWebhook(id, req.user.userId);
    return { message: 'Webhook deleted successfully' };
  }

  @Post(':id/test')
  @ApiOperation({ summary: 'Test a webhook' })
  @ApiResponse({ status: 200, description: 'Webhook test initiated' })
  async testWebhook(
    @Param('id') id: string,
    @Body() testDto?: TestWebhookDto,
    @Request() req
  ) {
    await this.webhookService.testWebhook(id, req.user.userId, testDto);
    return { message: 'Webhook test initiated' };
  }

  @Get('stats/overview')
  @ApiOperation({ summary: 'Get webhook notification statistics' })
  @ApiResponse({ status: 200, description: 'Statistics retrieved successfully' })
  async getNotificationStats(@Request() req) {
    return await this.webhookService.getNotificationStats(req.user.userId);
  }
}
