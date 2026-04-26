'use strict';

/**
 * Error logging service for production monitoring.
 * Supports pluggable transports (console by default; Sentry-compatible interface).
 */

const LOG_LEVELS = /** @type {const} */ (['debug', 'info', 'warn', 'error', 'fatal']);

/** @typedef {'debug'|'info'|'warn'|'error'|'fatal'} LogLevel */

/**
 * @typedef {Object} ErrorLogEntry
 * @property {LogLevel} level
 * @property {string} message
 * @property {unknown} [error]
 * @property {Record<string, unknown>} [context]
 * @property {string} timestamp
 */

/**
 * @typedef {Object} Transport
 * @property {(entry: ErrorLogEntry) => void} log
 */

class ConsoleTransport {
  /** @param {ErrorLogEntry} entry */
  log(entry) {
    const prefix = `[${entry.timestamp}] [${entry.level.toUpperCase()}]`;
    const msg = `${prefix} ${entry.message}`;
    if (entry.level === 'error' || entry.level === 'fatal') {
      // eslint-disable-next-line no-console
      console.error(msg, entry.error ?? '', entry.context ?? '');
    } else if (entry.level === 'warn') {
      // eslint-disable-next-line no-console
      console.warn(msg, entry.context ?? '');
    } else {
      // eslint-disable-next-line no-console
      console.log(msg, entry.context ?? '');
    }
  }
}

class ErrorLogger {
  /** @param {{ transports?: Transport[], minLevel?: LogLevel }} [options] */
  constructor(options = {}) {
    this._transports = options.transports ?? [new ConsoleTransport()];
    this._minLevel = options.minLevel ?? 'info';
  }

  /** @param {Transport} transport */
  addTransport(transport) {
    this._transports.push(transport);
  }

  /**
   * @param {LogLevel} level
   * @param {string} message
   * @param {{ error?: unknown, context?: Record<string, unknown> }} [meta]
   */
  _log(level, message, meta = {}) {
    if (LOG_LEVELS.indexOf(level) < LOG_LEVELS.indexOf(this._minLevel)) {
      return;
    }
    /** @type {ErrorLogEntry} */
    const entry = {
      level,
      message,
      error: meta.error,
      context: meta.context,
      timestamp: new Date().toISOString(),
    };
    for (const transport of this._transports) {
      try {
        transport.log(entry);
      } catch (_e) {
        // transport failure must not crash the app
      }
    }
  }

  /** @param {string} message @param {Record<string, unknown>} [context] */
  debug(message, context) { this._log('debug', message, { context }); }

  /** @param {string} message @param {Record<string, unknown>} [context] */
  info(message, context) { this._log('info', message, { context }); }

  /** @param {string} message @param {Record<string, unknown>} [context] */
  warn(message, context) { this._log('warn', message, { context }); }

  /**
   * @param {string} message
   * @param {unknown} error
   * @param {Record<string, unknown>} [context]
   */
  error(message, error, context) { this._log('error', message, { error, context }); }

  /**
   * @param {string} message
   * @param {unknown} error
   * @param {Record<string, unknown>} [context]
   */
  fatal(message, error, context) { this._log('fatal', message, { error, context }); }

  /**
   * Capture an unhandled exception.
   * @param {unknown} error
   * @param {Record<string, unknown>} [context]
   */
  captureException(error, context) {
    const message = error instanceof Error ? error.message : String(error);
    this._log('error', message, { error, context });
  }
}

const logger = new ErrorLogger();

module.exports = { ErrorLogger, ConsoleTransport, logger };
