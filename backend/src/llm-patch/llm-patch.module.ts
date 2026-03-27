import { Module } from '@nestjs/common';
import { HttpModule } from '@nestjs/axios';
import { ConfigModule } from '@nestjs/config';
import { LlmPatchController } from './llm-patch.controller';
import { LlmPatchService } from './llm-patch.service';

@Module({
  imports: [
    HttpModule,
    ConfigModule,
  ],
  controllers: [LlmPatchController],
  providers: [LlmPatchService],
  exports: [LlmPatchService],
})
export class LlmPatchModule {}
