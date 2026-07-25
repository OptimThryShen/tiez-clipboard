import { Fragment, useState, useEffect, useRef } from "react";
import type { ReactNode, MouseEvent as ReactMouseEvent } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
    Plus,
    Maximize2,
    Minimize2,
    ExternalLink,
    Folder,
    Download,
    Image as ImageIcon,
    Link as LinkIcon,
    Clipboard,
    Video,
    Send,
    Wifi,
    FileText,
    LoaderCircle,
    QrCode,
    FileArchive,
    Music,
    FileCode,
    Cpu,
    FileSpreadsheet,
    Presentation,
    File as FileIcon,
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { QRCodeCanvas } from "qrcode.react";
import type {
    FileTransferChatViewProps,
    FileTransferContextMenu,
    FileTransferMessage,
    FileTransferDevice
} from "../types";
import { getFileIcon as getSystemFileIcon, peekFileIcon } from "../../../shared/lib/fileIcon";
import { withNativeDialog } from "../../../shared/lib/focus";

type TransferFileKind =
    | 'archive'
    | 'audio'
    | 'executable'
    | 'pdf'
    | 'document'
    | 'spreadsheet'
    | 'presentation'
    | 'code'
    | 'image'
    | 'video'
    | 'file';

const getTransferFileKind = (fileName: string): TransferFileKind => {
    const extension = fileName.split('.').pop()?.toLowerCase() || '';
    if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'].includes(extension)) return 'archive';
    if (['mp3', 'wav', 'flac', 'm4a', 'aac', 'ogg'].includes(extension)) return 'audio';
    if (['exe', 'msi', 'bat', 'sh', 'app', 'dmg', 'pkg'].includes(extension)) return 'executable';
    if (extension === 'pdf') return 'pdf';
    if (['doc', 'docx', 'txt', 'rtf', 'md'].includes(extension)) return 'document';
    if (['xls', 'xlsx', 'csv', 'numbers'].includes(extension)) return 'spreadsheet';
    if (['ppt', 'pptx', 'key'].includes(extension)) return 'presentation';
    if (['js', 'ts', 'tsx', 'jsx', 'py', 'rs', 'c', 'cpp', 'go', 'java', 'html', 'css', 'json', 'yaml', 'yml', 'toml', 'sql'].includes(extension)) return 'code';
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'heic', 'avif'].includes(extension)) return 'image';
    if (['mp4', 'mov', 'avi', 'mkv', 'webm', 'm4v'].includes(extension)) return 'video';
    return 'file';
};

const TransferFileTypeIcon = ({
    filePath,
    fileName,
    preparing
}: {
    filePath?: string;
    fileName: string;
    preparing?: boolean;
}) => {
    const [systemIcon, setSystemIcon] = useState<string | null>(() => peekFileIcon(filePath) ?? null);
    const kind = getTransferFileKind(fileName);

    useEffect(() => {
        let cancelled = false;
        if (!filePath) {
            setSystemIcon(null);
            return () => { cancelled = true; };
        }

        const cached = peekFileIcon(filePath);
        if (cached !== undefined) {
            setSystemIcon(cached ?? null);
            return () => { cancelled = true; };
        }

        setSystemIcon(null);
        getSystemFileIcon(filePath).then(icon => {
            if (!cancelled) setSystemIcon(icon);
        });
        return () => { cancelled = true; };
    }, [filePath]);

    if (preparing) return <LoaderCircle size={22} className="wt-spin" />;
    if (systemIcon) {
        return <img src={systemIcon} alt="" className="wt-file-system-icon" loading="lazy" />;
    }

    const iconByKind: Record<TransferFileKind, ReactNode> = {
        archive: <FileArchive size={23} />,
        audio: <Music size={23} />,
        executable: <Cpu size={23} />,
        pdf: <FileText size={23} />,
        document: <FileText size={23} />,
        spreadsheet: <FileSpreadsheet size={23} />,
        presentation: <Presentation size={23} />,
        code: <FileCode size={23} />,
        image: <ImageIcon size={23} />,
        video: <Video size={23} />,
        file: <FileIcon size={23} />
    };

    return <span className={`wt-file-fallback-icon is-${kind}`}>{iconByKind[kind]}</span>;
};

const isLocalTransferPath = (path?: string) => {
    if (!path || /^(?:https?:|data:|blob:|asset:|file:|\/download\/)/i.test(path)) {
        return false;
    }
    return /^[a-z]:[\\/]/i.test(path) || /^\\\\/.test(path) || path.startsWith('/');
};

const decodeFileName = (value: string) => {
    try {
        return decodeURIComponent(value.replace(/\+/g, ' '));
    } catch {
        return value;
    }
};

const getTransferFileName = (message: FileTransferMessage) => {
    if (message._fileName?.trim()) return message._fileName.trim();

    for (const value of [message.content, message.file_path]) {
        if (!value) continue;
        const queryName = value.match(/(?:^|[?&])name=([^&]+)/i)?.[1];
        if (queryName) return decodeFileName(queryName);

        const withoutQuery = value.split(/[?#]/)[0];
        const baseName = withoutQuery.split(/[/\\]/).pop();
        if (baseName && !['download', 'file'].includes(baseName.toLowerCase())) {
            return decodeFileName(baseName);
        }
    }

    return '文件';
};

const getTransferFileIconPath = (message: FileTransferMessage) => {
    if (isLocalTransferPath(message.file_path)) return message.file_path;
    if (isLocalTransferPath(message.content)) return message.content;
    return undefined;
};

// File Transfer Chat View Component
const FileTransferChatView = ({
    t,
    localIp,
    actualPort,
    accessToken
}: FileTransferChatViewProps) => {
    const composerMinHeight = 32;
    const connectionUrl = `http://${localIp}:${actualPort}/?auth=${encodeURIComponent(accessToken)}`;
    const [messages, setMessages] = useState<FileTransferMessage[]>([]);
    const [mediaFallbackSources, setMediaFallbackSources] = useState<Record<string, string>>({});
    const [input, setInput] = useState("");
    const [appLogo, setAppLogo] = useState("");
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const chatBoxRef = useRef<HTMLDivElement>(null);
    const [isUserScrolling, setIsUserScrolling] = useState(false);
    const prevMessagesLengthRef = useRef(0);
    const [showFullScreen, setShowFullScreen] = useState(false);
    const [showExpandBtn, setShowExpandBtn] = useState(false);
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const contextMenuRef = useRef<HTMLDivElement>(null);
    const [contextMenu, setContextMenu] = useState<FileTransferContextMenu | null>(null);
    const [onlineDevices, setOnlineDevices] = useState<FileTransferDevice[]>([]);
    const [isDragging, setIsDragging] = useState(false);
    const [showQrCode, setShowQrCode] = useState(false);
    const [isChoosingFiles, setIsChoosingFiles] = useState(false);
    const hasOnlineDevices = onlineDevices.length > 0;
    const primaryDeviceName = typeof onlineDevices[0]?.name === 'string' && onlineDevices[0].name.trim()
        ? onlineDevices[0].name.trim()
        : '已连接设备';
    const conversationTitle = onlineDevices.length > 1
        ? `${primaryDeviceName} 等 ${onlineDevices.length} 台设备`
        : primaryDeviceName;

    const URL_REGEX = /((https?:\/\/|www\.)[^\s<]+|(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+(?:[a-z]{2,})(?:\/[^\s<]*)?)/gi;

    type PendingTransferItem = {
        name: string;
        path?: string;
        file?: File;
    };

    const resolveDropPaths = (payload: unknown): string[] => {
        if (Array.isArray(payload)) {
            return payload.filter((p): p is string => typeof p === "string" && p.trim().length > 0);
        }
        if (payload && typeof payload === "object" && "paths" in payload) {
            const maybePaths = (payload as { paths?: unknown }).paths;
            if (Array.isArray(maybePaths)) {
                return maybePaths.filter((p): p is string => typeof p === "string" && p.trim().length > 0);
            }
        }
        return [];
    };

    const getFilesFromDataTransfer = (dt: DataTransfer | null): File[] => {
        if (!dt) return [];
        const files: File[] = [];
        if (dt.items) {
            for (let i = 0; i < dt.items.length; i++) {
                const item = dt.items[i];
                if (item.kind !== "file") continue;
                const file = item.getAsFile();
                if (file) {
                    files.push(file);
                }
            }
            if (files.length > 0) {
                return files;
            }
        }
        if (dt.files && dt.files.length > 0) {
            return Array.from(dt.files);
        }
        return [];
    };

    const getDroppedTransferItems = (dt: DataTransfer | null): PendingTransferItem[] => {
        const files = getFilesFromDataTransfer(dt);
        const seen = new Set<string>();
        const items: PendingTransferItem[] = [];

        files.forEach((file) => {
            const maybePath = (file as File & { path?: string }).path?.trim();
            const key = maybePath || `${file.name}:${file.size}:${file.lastModified}`;
            if (seen.has(key)) return;
            seen.add(key);
            items.push({
                name: file.name || maybePath?.split(/[/\\]/).pop() || "File",
                path: maybePath || undefined,
                file: maybePath ? undefined : file
            });
        });

        return items;
    };

    const uploadDroppedFile = async (file: File) => {
        const port = Number(actualPort);
        if (!Number.isFinite(port) || port <= 0) {
            throw new Error("File transfer server is not running");
        }

        const CHUNK_SIZE = 1024 * 512;
        const totalChunks = Math.max(1, Math.ceil(file.size / CHUNK_SIZE));
        const uploadId = `desktop_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;

        for (let i = 0; i < totalChunks; i++) {
            const start = i * CHUNK_SIZE;
            const end = Math.min(file.size, start + CHUNK_SIZE);
            const chunk = file.slice(start, end);
            const formData = new FormData();
            formData.append("file", chunk, file.name);
            formData.append("metadata", JSON.stringify({
                upload_id: uploadId,
                chunk_index: i,
                total_chunks: totalChunks,
                file_name: file.name,
                sender_id: "pc",
                sender_name: "电脑",
                total_size: file.size,
                content_type: file.type || "application/octet-stream"
            }));

            const response = await fetch(`http://127.0.0.1:${port}/share-chunk`, {
                method: "POST",
                headers: { Authorization: `Bearer ${accessToken}` },
                body: formData
            });

            if (!response.ok) {
                const errorText = await response.text().catch(() => "");
                throw new Error(errorText || `Share chunk upload failed: ${response.status}`);
            }
        }
    };

    const queueFilesForSending = async (rawItems: PendingTransferItem[]) => {
        const items = rawItems.filter((item) => {
            if (item.path && item.path.trim().length > 0) return true;
            return !!item.file;
        });
        if (items.length === 0) return;

        const tempMessages: FileTransferMessage[] = items.map((item) => ({
            id: Date.now() + Math.random(),
            direction: 'out',
            msg_type: 'file',
            content: 'Preparing...',
            timestamp: Date.now(),
            _fileName: item.name,
            _preparing: true
        }));

        setMessages(prev => [...prev, ...tempMessages]);
        setTimeout(() => messagesEndRef.current?.scrollIntoView({ behavior: "smooth" }), 100);

        const results = await Promise.allSettled(
            items.map((item) => {
                if (item.path) {
                    return invoke("send_file_to_client", { filePath: item.path });
                }
                if (item.file) {
                    return uploadDroppedFile(item.file);
                }
                return Promise.reject(new Error("No valid file source"));
            })
        );

        const failedIds = new Set<number>();
        results.forEach((result, index) => {
            if (result.status === "rejected") {
                failedIds.add(tempMessages[index].id);
                console.error(`Failed to send file: ${items[index].name}`, result.reason);
            }
        });

        if (failedIds.size > 0) {
            setMessages(prev => prev.filter((msg) => !failedIds.has(msg.id)));
        }

        if (results.some((result) => result.status === "fulfilled")) {
            setTimeout(fetchMessages, 300);
        }
    };

    const normalizeUrl = (raw: string) => {
        if (/^https?:\/\//i.test(raw)) return raw;
        return `http://${raw}`;
    };

    const formatHttpHost = (host: string) => (
        host.includes(':') && !host.startsWith('[') ? `[${host}]` : host
    );

    const resolveShareableDownloadUrl = (content: string) => {
        if (!content.startsWith('/download/')) return content;
        if (localIp && actualPort) {
            return `http://${formatHttpHost(localIp)}:${actualPort}${content}`;
        }
        return content;
    };

    const resolveDesktopDownloadUrl = (content: string) => {
        if (!content.startsWith('/download/') || !actualPort) return content;
        return `http://127.0.0.1:${actualPort}${content}`;
    };

    const mediaFallbackKey = (message: FileTransferMessage) => (
        `${message.id}:${message.content}:${message.file_path || ''}`
    );

    const getMessageMediaSrc = (message: FileTransferMessage) => {
        const fallback = mediaFallbackSources[mediaFallbackKey(message)];
        if (fallback) return fallback;
        if (message.content.startsWith('data:')) return message.content;
        if (message.content.startsWith('/download/')) {
            return resolveDesktopDownloadUrl(message.content);
        }
        if (isLocalTransferPath(message.file_path)) return convertFileSrc(message.file_path!);
        if (isLocalTransferPath(message.content)) return convertFileSrc(message.content);
        return message.content;
    };

    const handleMediaPreviewError = async (
        message: FileTransferMessage,
        target: HTMLImageElement | HTMLVideoElement
    ) => {
        if (target.dataset.fallbackAttempted === 'true') return;
        target.dataset.fallbackAttempted = 'true';

        const localPath = [message.file_path, message.content]
            .find((value) => isLocalTransferPath(value));
        let fallbackPath: string | undefined;

        // Refresh the capability URL when a local path is available. Besides
        // avoiding Windows asset-protocol path quirks, this also repairs stale
        // preview tokens after the transfer server has restarted.
        if (localPath) {
            try {
                fallbackPath = await invoke<string>('get_download_url', { filePath: localPath });
            } catch (error) {
                console.error('Failed to create media preview URL', error);
            }
        }
        fallbackPath ||= message.content.startsWith('/download/')
            ? message.content
            : undefined;
        if (!fallbackPath) return;

        const fallbackSource = resolveDesktopDownloadUrl(fallbackPath);
        if (!/^http:\/\/127\.0\.0\.1:/i.test(fallbackSource)) return;
        setMediaFallbackSources((current) => ({
            ...current,
            [mediaFallbackKey(message)]: fallbackSource
        }));
    };

    const openTransferContent = async ({
        filePath,
        content,
        type
    }: {
        filePath?: string;
        content?: string;
        type?: string;
    }) => {
        const localTarget = [filePath, content].find(value => isLocalTransferPath(value));
        const rawTarget = localTarget || filePath || content;
        if (!rawTarget) {
            await emit('toast', '没有可打开的文件');
            return;
        }

        const isRemoteTarget = !localTarget && /^(?:https?:|\/download\/)/i.test(rawTarget);
        const target = isRemoteTarget ? resolveShareableDownloadUrl(rawTarget) : rawTarget;

        try {
            // File-transfer message IDs are session-local sequence numbers, not
            // clipboard database IDs. Passing them to open_content can replace
            // the supplied path with an unrelated clipboard entry.
            await invoke('open_content', {
                id: 0,
                content: target,
                contentType: isRemoteTarget ? 'url' : (type || 'file')
            });
        } catch (error) {
            console.error('Failed to open transferred content', error);
            await emit('toast', '无法打开此内容，请确认文件仍然存在');
        }
    };

    const normalizeTimestamp = (timestamp: number) => (
        timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp
    );

    const formatMessageTime = (timestamp: number) => {
        if (!timestamp) return '';
        const normalized = normalizeTimestamp(timestamp);
        const date = new Date(normalized);
        if (Number.isNaN(date.getTime())) return '';
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    };

    const getMessageDayKey = (timestamp: number) => {
        if (!timestamp) return '';
        const date = new Date(normalizeTimestamp(timestamp));
        if (Number.isNaN(date.getTime())) return '';
        return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
    };

    const formatMessageDay = (timestamp: number) => {
        const date = new Date(normalizeTimestamp(timestamp));
        const today = new Date();
        const yesterday = new Date(today);
        yesterday.setDate(today.getDate() - 1);

        const key = getMessageDayKey(timestamp);
        if (key === getMessageDayKey(today.getTime())) return '今天';
        if (key === getMessageDayKey(yesterday.getTime())) return '昨天';
        return date.toLocaleDateString([], {
            month: 'numeric',
            day: 'numeric',
            ...(date.getFullYear() === today.getFullYear() ? {} : { year: 'numeric' })
        });
    };

    const messagesBelongToSameGroup = (
        previous: FileTransferMessage | undefined,
        next: FileTransferMessage | undefined
    ) => {
        if (!previous || !next || previous.direction !== next.direction) return false;

        const previousSender = previous.sender_id || previous.sender_name || previous.direction;
        const nextSender = next.sender_id || next.sender_name || next.direction;
        if (previousSender !== nextSender) return false;

        const previousTime = normalizeTimestamp(previous.timestamp);
        const nextTime = normalizeTimestamp(next.timestamp);
        return Math.abs(nextTime - previousTime) <= 5 * 60 * 1000;
    };

    const openContextMenu = (
        event: ReactMouseEvent<HTMLElement>,
        menu: Omit<FileTransferContextMenu, 'x' | 'y'>
    ) => {
        event.preventDefault();
        event.stopPropagation();
        setContextMenu({
            ...menu,
            x: event.clientX,
            y: event.clientY
        });
    };

    const isLocalFilePath = isLocalTransferPath;

    const splitTrailingPunctuation = (raw: string) => {
        let url = raw;
        let trailing = '';
        while (url.length > 0 && /[)\]}>.,;!?]$/.test(url)) {
            trailing = url.slice(-1) + trailing;
            url = url.slice(0, -1);
        }
        return { url, trailing };
    };

    const renderTextWithLinks = (text: string) => {
        const parts: ReactNode[] = [];
        let lastIndex = 0;

        text.replace(URL_REGEX, (match, _group, _proto, offset) => {
            if (match.includes('@')) {
                return match;
            }
            const prevChar = offset > 0 ? text[offset - 1] : '';
            if (prevChar && /[a-z0-9@]/i.test(prevChar)) {
                return match;
            }
            if (offset > lastIndex) {
                parts.push(text.slice(lastIndex, offset));
            }

            const { url, trailing } = splitTrailingPunctuation(match);
            const href = normalizeUrl(url);

            parts.push(
                <a
                    key={`link-${offset}`}
                    href={href}
                    className="wt-link"
                    onClick={(e) => {
                        e.preventDefault();
                        if (window.getSelection()?.toString()) return;
                        invoke('open_content', { id: 0, content: href, contentType: 'url' }).catch(console.error);
                    }}
                >
                    {url}
                </a>
            );

            if (trailing) {
                parts.push(trailing);
            }

            lastIndex = offset + match.length;
            return match;
        });

        if (lastIndex < text.length) {
            parts.push(text.slice(lastIndex));
        }

        return parts.length > 0 ? parts : text;
    };

    useEffect(() => {
        invoke("set_navigation_enabled", { enabled: false }).catch(console.error);
        return () => {
            invoke("set_navigation_enabled", { enabled: true }).catch(console.error);
        };
    }, []);

    useEffect(() => {
        const handleCopy = (e: KeyboardEvent) => {
            const isCopy = (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'c';
            if (!isCopy) return;

            const selection = window.getSelection();
            const text = selection?.toString() || '';
            if (!text) return;

            const root = chatBoxRef.current;
            const anchor = selection?.anchorNode;
            const focus = selection?.focusNode;
            if (!root || !anchor || !focus) return;
            if (!root.contains(anchor) || !root.contains(focus)) return;

            e.preventDefault();
            e.stopPropagation();
            navigator.clipboard.writeText(text).catch(console.error);
        };

        window.addEventListener('keydown', handleCopy, true);
        return () => window.removeEventListener('keydown', handleCopy, true);
    }, []);

    const getAvatarConfig = (m: FileTransferMessage) => {
        if (m.sender_id === 'pc' || m.direction === 'out') {
            return { isImg: !!appLogo, content: appLogo || 'PC', color: 'var(--accent-color)', initial: 'PC' };
        }

        let initial = 'M';
        if (m.sender_name) {
            const name = m.sender_name.toLowerCase();
            if (name.includes('iphone')) initial = 'iP';
            else if (name.includes('ipad')) initial = 'iD';
            else if (name.includes('android')) initial = 'An';
            else if (name.includes('手机')) initial = 'M';
            else initial = m.sender_name.charAt(0).toUpperCase();
        }

        return {
            isImg: false,
            color: 'color-mix(in srgb, var(--accent-color) 74%, var(--text-secondary))',
            initial
        };
    };

    const fetchMessages = async () => {
        try {
            const msgs = await invoke<FileTransferMessage[]>("get_chat_history");
            setMessages(msgs);
        } catch (e) { }
    };

    useEffect(() => {
        console.log("FileTransferChatView Mounted - initializing listeners (Dual Mode)");
        const appWindow = getCurrentWindow();
        console.log("[DEBUG] appWindow label:", appWindow.label);
        fetchMessages();
        invoke<string>("get_app_logo").then(setAppLogo).catch(console.error);

        // Define handlers to be reused
        const handleDragDrop = (event: { payload: unknown }) => {
            console.log("[DRAG] Drop event received:", event);
            console.log("[DRAG] Event payload type:", typeof event.payload);
            console.log("[DRAG] Event payload:", JSON.stringify(event.payload, null, 2));
            setIsDragging(false);

            const paths = resolveDropPaths(event.payload);

            console.log("[DRAG] Parsed paths:", paths);

            if (paths && paths.length > 0) {
                void queueFilesForSending(
                    paths.map((path) => ({
                        name: path.split(/[/\\]/).pop() || "File",
                        path
                    }))
                );
            }
        };

        const handleDragEnter = (event: { payload: unknown }) => {
            console.log("[DRAG] Enter event received:", event);
            setIsDragging(true);
        };

        const handleDragLeave = (event: { payload: unknown }) => {
            console.log("[DRAG] Leave event received:", event);
            setIsDragging(false);
        };

        // Listen to BOTH v1 and v2 events just to be safe
        console.log("[DEBUG] Registering drag-drop event listeners...");

        // v1 event names (some versions still use these)
        const unlistenV1Drop = appWindow.listen("tauri://file-drop", (e) => {
            console.log("[v1] file-drop received");
            handleDragDrop(e);
        });
        const unlistenV1Hover = appWindow.listen("tauri://file-drop-hover", (e) => {
            console.log("[v1] file-drop-hover received");
            handleDragEnter(e);
        });
        const unlistenV1Cancel = appWindow.listen("tauri://file-drop-cancelled", (e) => {
            console.log("[v1] file-drop-cancelled received");
            handleDragLeave(e);
        });

        // v2 event names
        const unlistenV2Drop = appWindow.listen("tauri://drag-drop", (e) => {
            console.log("[v2] drag-drop received");
            handleDragDrop(e);
        });
        const unlistenV2Enter = appWindow.listen("tauri://drag-enter", (e) => {
            console.log("[v2] drag-enter received");
            handleDragEnter(e);
        });
        const unlistenV2Leave = appWindow.listen("tauri://drag-leave", (e) => {
            console.log("[v2] drag-leave received");
            handleDragLeave(e);
        });

        console.log("[DEBUG] All drag-drop listeners registered successfully");

        const unlistenDevices = listen<FileTransferDevice[]>("online-devices-updated", (event) => {
            setOnlineDevices(event.payload || []);
        });

        const unlistenNewMsg = listen<FileTransferMessage>("new-chat-message", () => {
            fetchMessages();
        });

        return () => {

            unlistenV1Drop.then(f => f());
            unlistenV1Hover.then(f => f());
            unlistenV1Cancel.then(f => f());
            unlistenV2Drop.then(f => f());
            unlistenV2Enter.then(f => f());
            unlistenV2Leave.then(f => f());
            unlistenDevices.then(f => f());
            unlistenNewMsg.then(f => f());
        };
    }, []);

    // Detect if user is at bottom
    useEffect(() => {
        const chatBox = chatBoxRef.current;
        if (!chatBox) return;

        const handleScroll = () => {
            const { scrollTop, scrollHeight, clientHeight } = chatBox;
            const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
            setIsUserScrolling(!isAtBottom);
        };

        chatBox.addEventListener('scroll', handleScroll);
        return () => chatBox.removeEventListener('scroll', handleScroll);
    }, []);

    // Smart Scroll
    useEffect(() => {
        const hasNewMessages = messages.length > prevMessagesLengthRef.current;
        prevMessagesLengthRef.current = messages.length;

        if (hasNewMessages && !isUserScrolling) {
            messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
        }
    }, [messages, isUserScrolling]);

    // Adjust textarea height
    useEffect(() => {
        if (textareaRef.current) {
            textareaRef.current.style.height = `${composerMinHeight}px`;
            const scrollHeight = textareaRef.current.scrollHeight;

            // Check if text is overflowing (more content than fits in max height)
            if (scrollHeight > 120) {
                setShowExpandBtn(true);
            } else {
                setShowExpandBtn(false);
            }

            textareaRef.current.style.height =
                Math.min(Math.max(composerMinHeight, scrollHeight), 120) + 'px';
        }
    }, [composerMinHeight, input]);

    const chooseFilesToSend = async () => {
        if (isChoosingFiles) return;
        setIsChoosingFiles(true);

        try {
            const selected = await withNativeDialog(() => open({
                multiple: true,
                title: '选择要发送的文件'
            }), 'file-transfer:choose-files');

            if (!selected) return;
            const paths = Array.isArray(selected) ? selected : [selected];
            await queueFilesForSending(
                paths.map(path => ({
                    name: path.split(/[/\\]/).pop() || '文件',
                    path
                }))
            );
        } catch (error) {
            console.error('Failed to choose files', error);
            await emit('toast', '无法打开文件选择器，请重试');
        } finally {
            setIsChoosingFiles(false);
        }
    };

    const send = async () => {
        if (!input.trim()) return;
        try {
            await invoke("send_chat_message", { msgType: "text", content: input });
            setInput("");
            setShowFullScreen(false);
            fetchMessages();
            // Reset height
            if (textareaRef.current) textareaRef.current.style.height = `${composerMinHeight}px`;
        } catch (e) { }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            send();
        }
    };

    const handlePaste = async (e: React.ClipboardEvent) => {
        if (e.clipboardData.files.length > 0) {
            const files = Array.from(e.clipboardData.files);
            const imageFiles = files.filter(f => f.type.startsWith('image/'));

            if (imageFiles.length > 0) {
                e.preventDefault();

                for (const file of imageFiles) {
                    const reader = new FileReader();
                    reader.onload = async (ev) => {
                        const base64 = ev.target?.result as string;
                        if (base64) {
                            try {
                                const savedPath = await invoke<string>("save_temp_image", { base64Data: base64 });
                                await invoke("send_file_to_client", { filePath: savedPath });
                            } catch (err) {
                                console.error("Failed to paste image", err);
                            }
                        }
                    };
                    reader.readAsDataURL(file);
                }
            }
        }
    };

    // Keep the contextual menu inside the window and dismiss it like a native IM menu.
    useEffect(() => {
        if (!contextMenu) return;

        const frame = requestAnimationFrame(() => {
            const menu = contextMenuRef.current;
            if (!menu) return;

            const edge = 8;
            const nextX = Math.max(edge, Math.min(contextMenu.x, window.innerWidth - menu.offsetWidth - edge));
            const nextY = Math.max(edge, Math.min(contextMenu.y, window.innerHeight - menu.offsetHeight - edge));

            if (nextX !== contextMenu.x || nextY !== contextMenu.y) {
                setContextMenu(current => current ? { ...current, x: nextX, y: nextY } : current);
            }

            if (!menu.contains(document.activeElement)) menu.focus({ preventScroll: true });
        });

        const handlePointerDown = (event: PointerEvent) => {
            if (!contextMenuRef.current?.contains(event.target as Node)) {
                setContextMenu(null);
            }
        };
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                setContextMenu(null);
                return;
            }

            if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
            const items = Array.from(
                contextMenuRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') || []
            );
            if (items.length === 0) return;

            event.preventDefault();
            const activeIndex = items.indexOf(document.activeElement as HTMLElement);
            if (event.key === 'Home') items[0].focus();
            if (event.key === 'End') items[items.length - 1].focus();
            if (event.key === 'ArrowDown') items[activeIndex < 0 ? 0 : (activeIndex + 1) % items.length].focus();
            if (event.key === 'ArrowUp') items[activeIndex < 0 ? items.length - 1 : (activeIndex - 1 + items.length) % items.length].focus();
        };
        const closeMenu = () => setContextMenu(null);

        document.addEventListener('pointerdown', handlePointerDown, true);
        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('resize', closeMenu);
        window.addEventListener('scroll', closeMenu, true);

        return () => {
            cancelAnimationFrame(frame);
            document.removeEventListener('pointerdown', handlePointerDown, true);
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('resize', closeMenu);
            window.removeEventListener('scroll', closeMenu, true);
        };
    }, [contextMenu]);

    useEffect(() => {
        const handleWindowDragOver = (event: globalThis.DragEvent) => {
            event.preventDefault();
            if (event.dataTransfer) {
                event.dataTransfer.dropEffect = "copy";
            }
            if (!isDragging) {
                setIsDragging(true);
            }
        };

        const handleWindowDragLeave = (event: globalThis.DragEvent) => {
            if (event.relatedTarget === null) {
                setIsDragging(false);
            }
        };

        const handleWindowDrop = (event: globalThis.DragEvent) => {
            event.preventDefault();
            event.stopPropagation();
            setIsDragging(false);

            const items = getDroppedTransferItems(event.dataTransfer);
            if (items.length > 0) {
                void queueFilesForSending(items);
                return;
            }

            void emit("toast", "未识别到拖入文件，请重启应用后重试");
        };

        window.addEventListener("dragover", handleWindowDragOver);
        window.addEventListener("dragleave", handleWindowDragLeave);
        window.addEventListener("drop", handleWindowDrop);

        return () => {
            window.removeEventListener("dragover", handleWindowDragOver);
            window.removeEventListener("dragleave", handleWindowDragLeave);
            window.removeEventListener("drop", handleWindowDrop);
        };
    }, [isDragging]);

    return (
        <div
            className="wt-chat-view"
            style={{ position: 'relative' }}
            onDragOver={(e) => {
                // Critical: Prevent default browser behavior to allow drop
                e.preventDefault();
                if (e.dataTransfer) {
                    e.dataTransfer.dropEffect = 'copy';
                }
                if (!isDragging) setIsDragging(true);
            }}
            onDragLeave={(e) => {
                // Check if leaving the main container
                if (e.currentTarget.contains(e.relatedTarget as Node)) return;
                setIsDragging(false);
            }}
            onDrop={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setIsDragging(false);
                const items = getDroppedTransferItems(e.dataTransfer);
                if (items.length > 0) {
                    void queueFilesForSending(items);
                    return;
                }
                void emit("toast", "未识别到拖入文件，请重启应用后重试");
            }}
        >
            {/* Drag Overlay */}
            <AnimatePresence>
                {isDragging && (
                    <motion.div
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        className="wt-drag-overlay"
                    >
                        <div className="wt-drag-drop-zone">
                            <Folder size={64} strokeWidth={1.5} />
                            <div className="wt-drag-drop-text">松开发送文件</div>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>

            {localIp && actualPort && accessToken && (
                <div className={`wt-header ${hasOnlineDevices ? 'is-connected' : 'is-onboarding'}`}>
                    {!hasOnlineDevices ? (
                        <div className="wt-onboarding-connect">
                            <div className="wt-onboarding-identity">
                                <div className="wt-onboarding-copy">
                                    <div className="wt-onboarding-title">局域网文件传输</div>
                                    <div className="wt-onboarding-status">等待设备连接</div>
                                    <div className="wt-onboarding-address">{localIp}:{actualPort}</div>
                                </div>
                            </div>
                            <div className="wt-onboarding-qr" title="手机扫码连接">
                                <QRCodeCanvas value={connectionUrl} size={54} />
                            </div>
                        </div>
                    ) : (
                        <>
                            <div className="wt-conversation-identity">
                                <div className="wt-conversation-copy">
                                    <div className="wt-conversation-title">{conversationTitle}</div>
                                    <div className="wt-presence-line">
                                        {onlineDevices.length === 1 ? '在线 · 局域网传输' : `${onlineDevices.length} 台设备在线`}
                                    </div>
                                </div>
                            </div>
                            <button
                                type="button"
                                className={`wt-header-action ${showQrCode ? 'is-active' : ''}`}
                                title="显示连接二维码"
                                aria-label="显示连接二维码"
                                aria-expanded={showQrCode}
                                onClick={() => setShowQrCode(value => !value)}
                            >
                                <QrCode size={19} />
                            </button>
                            {showQrCode && (
                                <div className="wt-qr-popover">
                                    <div className="wt-qr-popover-code">
                                        <QRCodeCanvas value={connectionUrl} size={112} />
                                    </div>
                                    <div className="wt-qr-popover-title">连接其他设备</div>
                                    <div className="wt-qr-popover-address">{localIp}:{actualPort}</div>
                                </div>
                            )}
                        </>
                    )}
                </div>
            )}

            <div className="wt-chat-box" ref={chatBoxRef}>
                {messages.length === 0 && (
                    <div className="wt-empty-state">
                        <div className={`wt-empty-icon${hasOnlineDevices ? '' : ' is-waiting'}`} aria-hidden="true">
                            <Wifi size={22} strokeWidth={1.8} />
                        </div>
                        <div className="wt-empty-title">{hasOnlineDevices ? '开始传输' : '等待设备连接'}</div>
                        <div className="wt-empty-copy">
                            {hasOnlineDevices ? '输入消息、选择文件，或直接将文件拖到这里' : '使用上方二维码连接手机后，即可互传消息与文件'}
                        </div>
                    </div>
                )}
                {messages.map((m, index) => {
                    const avatar = getAvatarConfig(m);
                    const transferFileName = getTransferFileName(m);
                    const groupedWithPrevious = messagesBelongToSameGroup(messages[index - 1], m);
                    const groupedWithNext = messagesBelongToSameGroup(m, messages[index + 1]);
                    const startsNewDay = index === 0
                        || getMessageDayKey(messages[index - 1].timestamp) !== getMessageDayKey(m.timestamp);
                    return (
                        <Fragment key={m.id}>
                        {startsNewDay && m.timestamp > 0 && (
                            <div className="wt-day-separator" role="separator">
                                <span>{formatMessageDay(m.timestamp)}</span>
                            </div>
                        )}
                        <div
                            className={`wt-message ${m.direction === 'out' ? 'sent' : 'received'} ${groupedWithPrevious ? 'is-grouped-with-previous' : 'is-group-start'} ${groupedWithNext ? 'is-grouped-with-next' : 'is-group-end'}`}
                        >
                            {!groupedWithNext ? (
                                <div
                                    className="wt-avatar"
                                    style={{
                                        background: avatar.isImg ? 'transparent' : avatar.color,
                                    }}
                                    aria-label={m.direction === 'out' ? '我的设备' : (m.sender_name || '对方设备')}
                                >
                                    {avatar.isImg ? (
                                        <img
                                            src={avatar.content}
                                            loading="lazy"
                                            alt={m.direction === 'out' ? '我的设备' : (m.sender_name || '对方设备')}
                                        />
                                    ) : (
                                        avatar.initial
                                    )}
                                </div>
                            ) : (
                                <div className="wt-avatar-spacer" aria-hidden="true" />
                            )}
                            <div className="wt-message-stack">
                                <div
                                    className={`wt-bubble wt-bubble-${m.msg_type}`}
                                    onContextMenu={(event) => {
                                        if (m._preparing) return;
                                        const menuType = m.msg_type === 'text' || m.msg_type === 'image' || m.msg_type === 'video'
                                            ? m.msg_type
                                            : 'file';
                                        openContextMenu(event, {
                                            filePath: menuType === 'text' ? undefined : (m.file_path || m.content),
                                            content: m.content,
                                            id: m.id,
                                            type: menuType
                                        });
                                    }}
                                >
                                {m.sender_name && m.direction === 'in' && !groupedWithPrevious && (
                                    <div className="wt-sender-name">{m.sender_name}</div>
                                )}
                                {m.msg_type === 'text' && (
                                    <div
                                        className="wt-text-content"
                                        onContextMenu={(e) => openContextMenu(e, {
                                                content: m.content,
                                                type: 'text'
                                            })}
                                    >{renderTextWithLinks(m.content)}</div>
                                )}
                                {m.msg_type === 'image' && (
                                    <>
                                        <img
                                            src={getMessageMediaSrc(m)}
                                            className="wt-img-preview"
                                            loading="lazy"
                                            style={{ cursor: 'pointer' }}
                                            alt="Image"
                                            onClick={async () => {
                                                await openTransferContent({
                                                    filePath: m.file_path,
                                                    content: m.content,
                                                    type: 'image'
                                                });
                                            }}
                                            onError={(event) => {
                                                void handleMediaPreviewError(m, event.currentTarget);
                                            }}
                                            onContextMenu={(e) => openContextMenu(e, {
                                                    filePath: m.file_path || m.content,
                                                    content: m.content,
                                                    id: m.id,
                                                    type: 'image'
                                                })}
                                        />
                                        <div className="wt-media-footer">
                                            <ImageIcon size={12} />
                                            <span>图片</span>
                                        </div>
                                    </>
                                )}
                                {m.msg_type === 'video' && (
                                    <>
                                        <video
                                            src={getMessageMediaSrc(m)}
                                            className="wt-video-preview"
                                            controls
                                            onError={(event) => {
                                                void handleMediaPreviewError(m, event.currentTarget);
                                            }}
                                            onContextMenu={(e) => openContextMenu(e, {
                                                    filePath: m.file_path || m.content,
                                                    content: m.content,
                                                    id: m.id,
                                                    type: 'video'
                                                })}
                                        />
                                        <div className="wt-media-footer">
                                            <Video size={12} />
                                            <span>视频</span>
                                        </div>
                                    </>
                                )}
                                {(m.msg_type === 'file' || (m.msg_type !== 'text' && m.msg_type !== 'image' && m.msg_type !== 'video')) && (
                                    <>
                                        <div className="wt-file-card"
                                            style={{ cursor: m.direction === 'in' && !m._preparing ? 'pointer' : 'default' }}
                                            onClick={async () => {
                                                if (m.direction === 'in' && !m._preparing) {
                                                    await openTransferContent({
                                                        filePath: m.file_path,
                                                        content: m.content,
                                                        type: 'file'
                                                    });
                                                }
                                            }}

                                            onContextMenu={(e) => {
                                                if (!m._preparing) {
                                                    openContextMenu(e, {
                                                        filePath: m.file_path || m.content,
                                                        content: m.content,
                                                        id: m.id,
                                                        type: 'file'
                                                    });
                                                }
                                            }}
                                        >
                                            <div className="wt-file-icon">
                                                <TransferFileTypeIcon
                                                    filePath={getTransferFileIconPath(m)}
                                                    fileName={transferFileName}
                                                    preparing={m._preparing}
                                                />
                                            </div>
                                            <div className="wt-file-info">
                                                <div className="wt-file-name">{transferFileName}</div>
                                                {!m._preparing && (
                                                    <div className="wt-file-status">
                                                        {m.direction === 'in' ? '已接收 · 点击打开' : '可供下载'}
                                                    </div>
                                                )}
                                            </div>
                                        </div>
                                        {m._preparing && (
                                            <div className="progress-wrapper">
                                                <div className="progress-container">
                                                    <div className="progress-bar" style={{ width: '100%', animation: 'pulse 1.5s ease-in-out infinite' }}></div>
                                                </div>
                                                <div className="status-text">
                                                    <span className="status-label">正在准备文件…</span>
                                                    <span className="percent"></span>
                                                </div>
                                            </div>
                                        )}
                                    </>
                                )}
                                </div>
                                <div className="wt-message-meta">
                                    <span>{formatMessageTime(m.timestamp)}</span>
                                    {m.direction === 'out' && <span className="wt-delivery-mark">✓</span>}
                                </div>
                            </div>
                        </div>
                        </Fragment>
                    );
                })}
                <div ref={messagesEndRef} />
            </div>

            <div className="wt-footer">
                <div className="wt-composer">
                    <button
                        type="button"
                        className="wt-btn-icon"
                        title={isChoosingFiles ? '正在打开文件选择器' : '发送文件'}
                        aria-label={isChoosingFiles ? '正在打开文件选择器' : '发送文件'}
                        disabled={isChoosingFiles}
                        onClick={() => void chooseFilesToSend()}
                    >
                        {isChoosingFiles
                            ? <LoaderCircle size={18} className="wt-spin" />
                            : <Plus size={18} />}
                    </button>

                    <div className="wt-input-wrap">
                        <textarea
                            ref={textareaRef}
                            className="wt-input"
                            value={input}
                            onChange={e => setInput(e.target.value)}
                            onKeyDown={handleKeyDown}
                            onPaste={handlePaste}
                            placeholder={t ? (t('type_message') || "Type a message...") : "Type..."}
                            rows={1}
                        />
                    </div>
                        {showExpandBtn && (
                            <button
                                type="button"
                                className="wt-btn-icon"
                                onClick={() => setShowFullScreen(true)}
                                title="展开编辑"
                                aria-label="展开编辑"
                            >
                                <Maximize2 size={16} />
                            </button>
                        )}

                    <button
                        type="button"
                        onClick={send}
                        className="wt-btn send"
                        disabled={!input.trim()}
                        title="发送消息"
                        aria-label="发送消息"
                    >
                        <Send size={18} />
                    </button>
                </div>
            </div>

            <AnimatePresence>
                {showFullScreen && (
                    <motion.div
                        initial={{ opacity: 0, scale: 0.95 }}
                        animate={{ opacity: 1, scale: 1 }}
                        exit={{ opacity: 0, scale: 0.95 }}
                        className="wt-fullscreen-editor"
                    >
                        <div className="wt-fullscreen-header">
                            <div className="wt-fullscreen-title">编辑消息</div>
                            <button
                                onClick={() => setShowFullScreen(false)}
                                className="wt-overlay-icon-btn"
                                title="收起编辑器"
                                aria-label="收起编辑器"
                            >
                                <Minimize2 size={16} />
                            </button>
                        </div>

                        <textarea
                            value={input}
                            onChange={e => setInput(e.target.value)}
                            placeholder="输入消息…"
                            className="wt-fullscreen-textarea"
                            onPaste={handlePaste}
                        />

                        <div className="wt-fullscreen-footer">
                            <button
                                onClick={() => setShowFullScreen(false)}
                                className="wt-btn"
                            >
                                取消
                            </button>
                            <button
                                onClick={send}
                                className="wt-btn send"
                            >
                                发送
                            </button>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>

            <AnimatePresence>
                {contextMenu && (
                    <motion.div
                        ref={contextMenuRef}
                        className="wt-context-menu"
                        role="menu"
                        aria-label="消息操作"
                        tabIndex={-1}
                        initial={{ opacity: 0, scale: 0.97 }}
                        animate={{ opacity: 1, scale: 1 }}
                        exit={{ opacity: 0, scale: 0.97 }}
                        transition={{ duration: 0.12 }}
                        style={{ top: contextMenu.y, left: contextMenu.x }}
                        onContextMenu={(event) => event.preventDefault()}
                    >
                        {(contextMenu.type === 'file' || contextMenu.type === 'image' || contextMenu.type === 'video') && (
                            <>
                                <button
                                    type="button"
                                    className="wt-context-item"
                                    role="menuitem"
                                    onClick={async () => {
                                        const selected = contextMenu;
                                        setContextMenu(null);
                                        await openTransferContent({
                                            filePath: selected.filePath,
                                            content: selected.content,
                                            type: selected.type
                                        });
                                    }}
                                >
                                    <ExternalLink size={16} />
                                    <span>{t ? (t('open') || '打开') : '打开'}</span>
                                </button>
                                {isLocalFilePath(contextMenu.filePath) && (
                                    <button
                                        type="button"
                                        className="wt-context-item"
                                        role="menuitem"
                                        onClick={async () => {
                                            const selected = contextMenu;
                                            setContextMenu(null);
                                            try {
                                                await invoke('open_file_location', { filePath: selected.filePath });
                                            } catch (error) {
                                                console.error('Failed to reveal file', error);
                                            }
                                        }}
                                    >
                                        <Folder size={16} />
                                        <span>{t ? (t('show_in_finder') || '在访达中显示') : '在访达中显示'}</span>
                                    </button>
                                )}
                                <div className="wt-context-separator" role="separator" />
                            </>
                        )}

                        {contextMenu.type === 'image' && (
                            <>
                                {isLocalFilePath(contextMenu.filePath) && (
                                    <button
                                        type="button"
                                        className="wt-context-item"
                                        role="menuitem"
                                        onClick={async () => {
                                            const selected = contextMenu;
                                            setContextMenu(null);
                                            try {
                                                const target = await withNativeDialog(() => saveDialog({
                                                    defaultPath: selected.filePath?.split(/[/\\]/).pop() || 'image.png',
                                                    filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
                                                }), 'file-transfer:save-image');
                                                if (target && selected.filePath) {
                                                    await invoke('save_file_copy', { sourcePath: selected.filePath, targetPath: target });
                                                }
                                            } catch (error) {
                                                console.error('Failed to save image', error);
                                            }
                                        }}
                                    >
                                        <Download size={16} />
                                        <span>{t ? (t('save_image_as') || '图片另存为…') : '图片另存为…'}</span>
                                    </button>
                                )}
                                <button
                                    type="button"
                                    className="wt-context-item"
                                    role="menuitem"
                                    onClick={async () => {
                                        const selected = contextMenu;
                                        setContextMenu(null);
                                        if (selected.filePath) {
                                            await invoke('copy_to_clipboard', { content: selected.filePath, contentType: 'image', paste: false, id: 0, deleteAfterUse: false })
                                                .catch(error => console.error('Failed to copy image', error));
                                        }
                                    }}
                                >
                                    <ImageIcon size={16} />
                                    <span>{t ? (t('copy_image') || '复制图片') : '复制图片'}</span>
                                </button>
                                <button
                                    type="button"
                                    className="wt-context-item"
                                    role="menuitem"
                                    onClick={async () => {
                                        const selected = contextMenu;
                                        setContextMenu(null);
                                        const link = resolveShareableDownloadUrl(selected.content || selected.filePath || '');
                                        if (link) await navigator.clipboard.writeText(link);
                                    }}
                                >
                                    <LinkIcon size={16} />
                                    <span>{t ? (t('copy_image_link') || '复制图片链接') : '复制图片链接'}</span>
                                </button>
                            </>
                        )}

                        {contextMenu.type === 'video' && (
                            <>
                                {isLocalFilePath(contextMenu.filePath) && (
                                    <button
                                        type="button"
                                        className="wt-context-item"
                                        role="menuitem"
                                        onClick={async () => {
                                            const selected = contextMenu;
                                            setContextMenu(null);
                                            try {
                                                const target = await withNativeDialog(() => saveDialog({
                                                    defaultPath: selected.filePath?.split(/[/\\]/).pop() || 'video.mp4',
                                                    filters: [{ name: '视频', extensions: ['mp4', 'mov', 'avi', 'mkv', 'webm'] }]
                                                }), 'file-transfer:save-video');
                                                if (target && selected.filePath) {
                                                    await invoke('save_file_copy', { sourcePath: selected.filePath, targetPath: target });
                                                }
                                            } catch (error) {
                                                console.error('Failed to save video', error);
                                            }
                                        }}
                                    >
                                        <Download size={16} />
                                        <span>{t ? (t('save_video_as') || '视频另存为…') : '视频另存为…'}</span>
                                    </button>
                                )}
                                <button
                                    type="button"
                                    className="wt-context-item"
                                    role="menuitem"
                                    onClick={async () => {
                                        const selected = contextMenu;
                                        setContextMenu(null);
                                        if (selected.filePath) {
                                            await invoke('copy_to_clipboard', { content: selected.filePath, contentType: 'video', paste: false, id: 0, deleteAfterUse: false })
                                                .catch(error => console.error('Failed to copy video', error));
                                        }
                                    }}
                                >
                                    <Video size={16} />
                                    <span>{t ? (t('copy_video') || '复制视频') : '复制视频'}</span>
                                </button>
                                <button
                                    type="button"
                                    className="wt-context-item"
                                    role="menuitem"
                                    onClick={async () => {
                                        const selected = contextMenu;
                                        setContextMenu(null);
                                        const link = resolveShareableDownloadUrl(selected.content || selected.filePath || '');
                                        if (link) await navigator.clipboard.writeText(link);
                                    }}
                                >
                                    <LinkIcon size={16} />
                                    <span>{t ? (t('copy_video_link') || '复制视频链接') : '复制视频链接'}</span>
                                </button>
                            </>
                        )}

                        {contextMenu.type === 'text' && (
                            <button
                                type="button"
                                className="wt-context-item"
                                role="menuitem"
                                onClick={async () => {
                                    const selected = contextMenu;
                                    setContextMenu(null);
                                    if (selected.content) await navigator.clipboard.writeText(selected.content);
                                }}
                            >
                                <Clipboard size={16} />
                                <span>{t ? (t('copy_text') || '复制文本') : '复制文本'}</span>
                            </button>
                        )}

                        {contextMenu.type === 'file' && (
                            <button
                                type="button"
                                className="wt-context-item"
                                role="menuitem"
                                onClick={async () => {
                                    const selected = contextMenu;
                                    setContextMenu(null);
                                    const link = resolveShareableDownloadUrl(selected.content || selected.filePath || '');
                                    if (link) await navigator.clipboard.writeText(link);
                                }}
                            >
                                <LinkIcon size={16} />
                                <span>{t ? (t('copy_link') || '复制下载链接') : '复制下载链接'}</span>
                            </button>
                        )}
                    </motion.div>
                )}
            </AnimatePresence>
        </div >
    );
};

export default FileTransferChatView;
