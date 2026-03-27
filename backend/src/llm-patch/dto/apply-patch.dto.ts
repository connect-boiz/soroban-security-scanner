import { IsString, IsNotEmpty } from 'class-validator';

export class ApplyPatchDto {
  @IsString()
  @IsNotEmpty()
  target_dir: string;
}
