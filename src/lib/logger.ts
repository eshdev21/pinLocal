import { info, error, warn, debug, trace } from '@tauri-apps/plugin-log';

/**
 * Centralized logger for PinLocal that prints to the Webview console,
 * streams to the global Tauri log (Stdout/app.log), and writes to custom webview.log.
 */
const formatMessage = (message: string, ...args: any[]): string => {
  if (args.length === 0) return message;
  return `${message} ${args.map(arg => {
    if (arg instanceof Error) return arg.stack || arg.message;
    if (typeof arg === 'object') {
      try {
        return JSON.stringify(arg);
      } catch {
        return String(arg);
      }
    }
    return String(arg);
  }).join(' ')}`;
};

export const logger = {
  info: (message: string, ...args: any[]) => {
    const formatted = formatMessage(message, ...args);
    console.info(`[INFO]`, message, ...args);
    info(formatted).catch(console.error);
  },
  error: (message: string, ...args: any[]) => {
    const formatted = formatMessage(message, ...args);
    console.error(`[ERROR]`, message, ...args);
    error(formatted).catch(console.error);
  },
  warn: (message: string, ...args: any[]) => {
    const formatted = formatMessage(message, ...args);
    console.warn(`[WARN]`, message, ...args);
    warn(formatted).catch(console.error);
  },
  debug: (message: string, ...args: any[]) => {
    const formatted = formatMessage(message, ...args);
    console.debug(`[DEBUG]`, message, ...args);
    debug(formatted).catch(console.error);
  },
  trace: (message: string, ...args: any[]) => {
    const formatted = formatMessage(message, ...args);
    console.trace(`[TRACE]`, message, ...args);
    trace(formatted).catch(console.error);
  }
};

