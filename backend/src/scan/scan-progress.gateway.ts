import {
  WebSocketGateway,
  WebSocketServer,
  SubscribeMessage,
  OnGatewayConnection,
  OnGatewayDisconnect,
  OnGatewayInit,
  ConnectedSocket,
  MessageBody,
} from '@nestjs/websockets';
import { Server, Socket } from 'socket.io';
import { Logger } from '@nestjs/common';
import { ScanService } from './scan.service';

@WebSocketGateway({
  cors: {
    origin: '*',
    methods: ['GET', 'POST'],
    credentials: true,
  },
  namespace: '/scan-progress',
})
export class ScanProgressGateway
  implements OnGatewayInit, OnGatewayConnection, OnGatewayDisconnect
{
  @WebSocketServer() server: Server;
  private logger: Logger = new Logger('ScanProgressGateway');

  constructor(private readonly scanService: ScanService) {}

  afterInit(server: Server) {
    this.logger.log('WebSocket Gateway initialized');
  }

  handleConnection(client: Socket, ...args: any[]) {
    this.logger.log(`Client connected: ${client.id}`);
  }

  handleDisconnect(client: Socket) {
    this.logger.log(`Client disconnected: ${client.id}`);
  }

  @SubscribeMessage('subscribe-scan')
  handleSubscribeScan(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { scanId: string },
  ) {
    const { scanId } = data;
    client.join(`scan-${scanId}`);
    this.logger.log(`Client ${client.id} subscribed to scan ${scanId}`);
    
    // Send current scan status if available
    this.scanService.getScan(scanId).then(scan => {
      if (scan) {
        client.emit('scan-status', {
          scanId,
          status: scan.status,
          progress: scan.progress || 0,
          currentStep: scan.currentStep || 'uploading',
          logs: scan.logs || [],
          vulnerabilities: scan.vulnerabilities || [],
        });
      }
    }).catch(error => {
      this.logger.error(`Error getting scan ${scanId}:`, error);
    });
  }

  @SubscribeMessage('unsubscribe-scan')
  handleUnsubscribeScan(
    @ConnectedSocket() client: Socket,
    @MessageBody() data: { scanId: string },
  ) {
    const { scanId } = data;
    client.leave(`scan-${scanId}`);
    this.logger.log(`Client ${client.id} unsubscribed from scan ${scanId}`);
  }

  // Methods to emit scan progress updates
  emitScanProgress(scanId: string, data: any) {
    this.server.to(`scan-${scanId}`).emit('scan-progress', {
      scanId,
      ...data,
    });
  }

  emitScanStatus(scanId: string, status: string) {
    this.server.to(`scan-${scanId}`).emit('scan-status', {
      scanId,
      status,
      timestamp: new Date().toISOString(),
    });
  }

  emitScanLog(scanId: string, log: string) {
    this.server.to(`scan-${scanId}`).emit('scan-log', {
      scanId,
      log,
      timestamp: new Date().toISOString(),
    });
  }

  emitScanComplete(scanId: string, results: any) {
    this.server.to(`scan-${scanId}`).emit('scan-complete', {
      scanId,
      results,
      timestamp: new Date().toISOString(),
    });
  }

  emitScanError(scanId: string, error: string) {
    this.server.to(`scan-${scanId}`).emit('scan-error', {
      scanId,
      error,
      timestamp: new Date().toISOString(),
    });
  }
}
