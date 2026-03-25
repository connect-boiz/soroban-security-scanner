import { 
  Controller, 
  Get, 
  Post, 
  Put, 
  Delete, 
  Body, 
  Param, 
  UseGuards, 
  Request,
  HttpCode,
  HttpStatus,
  ValidationPipe,
  ClassSerializerInterceptor,
  UseInterceptors,
} from '@nestjs/common';
import { ApiKeyService } from './api-key.service';
import { CreateApiKeyDto, UpdateApiKeyDto, ApiKeyResponseDto, GenerateApiKeyResponseDto } from './dto/api-key.dto';
import { JwtAuthGuard } from '../auth/jwt-auth.guard';
import { RolesGuard } from '../auth/roles.guard';
import { Roles } from '../auth/roles.decorator';

@Controller('api-keys')
@UseGuards(JwtAuthGuard, RolesGuard)
@UseInterceptors(ClassSerializerInterceptor)
export class ApiKeyController {
  constructor(private readonly apiKeyService: ApiKeyService) {}

  @Post()
  @Roles('admin', 'developer')
  @HttpCode(HttpStatus.CREATED)
  async generateKey(
    @Body(ValidationPipe) createApiKeyDto: CreateApiKeyDto,
    @Request() req: any,
  ): Promise<GenerateApiKeyResponseDto> {
    return this.apiKeyService.generateApiKey(createApiKeyDto, req.user.userId);
  }

  @Get()
  @Roles('admin', 'developer', 'viewer')
  async findAll(@Request() req: any): Promise<ApiKeyResponseDto[]> {
    return this.apiKeyService.findAllApiKeys(req.user.userId);
  }

  @Put(':id')
  @Roles('admin', 'developer')
  async update(
    @Param('id') id: string,
    @Body(ValidationPipe) updateApiKeyDto: UpdateApiKeyDto,
    @Request() req: any,
  ): Promise<ApiKeyResponseDto> {
    return this.apiKeyService.updateApiKey(id, updateApiKeyDto, req.user.userId);
  }

  @Delete(':id')
  @Roles('admin', 'developer')
  @HttpCode(HttpStatus.NO_CONTENT)
  async revoke(@Param('id') id: string, @Request() req: any): Promise<void> {
    await this.apiKeyService.revokeApiKey(id, req.user.userId);
  }
}
