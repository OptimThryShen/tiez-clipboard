export interface FileTransferChatViewProps {
  t: (key: string) => string;
  localIp: string;
  actualPort: string;
  accessToken: string;
}

export type FileTransferMessageDirection = "in" | "out";

export interface FileTransferMessage {
  id: number;
  direction: FileTransferMessageDirection;
  msg_type: string;
  content: string;
  timestamp: number;
  sender_id?: string;
  sender_name?: string;
  file_path?: string;
  batch_id?: string;
  batch_name?: string;
  batch_index?: number;
  batch_total?: number;
  batch_size?: number;
  file_size?: number;
  _preparing?: boolean;
  _fileName?: string;
}

export type FileTransferDragPayload = string[] | { paths: string[] };

export interface FileTransferDevice {
  id?: string;
  name?: string;
  [key: string]: unknown;
}

export interface FileTransferContextMenu {
  x: number;
  y: number;
  filePath?: string;
  content?: string;
  id?: number;
  type?: string;
  batchId?: string;
  batchMessages?: FileTransferMessage[];
}
