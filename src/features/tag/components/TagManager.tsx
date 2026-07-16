import { useState, useEffect, useRef, useMemo, useCallback, type MouseEvent as ReactMouseEvent } from 'react';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import {
    Edit2, Trash2, X, ChevronRight, LayoutGrid, List,
    Clock, MousePointer2, ChevronLeft, Plus, ExternalLink, CheckSquare, Copy, StickyNote
} from 'lucide-react';
import AppModal from "../../../shared/components/AppModal";
import { getTagColor } from "../../../shared/lib/utils";
import { toTauriLocalImageSrc, withImageCacheBust } from "../../../shared/lib/localImageSrc";
import type { ClipboardEntry } from "../../../shared/types";

interface TagManagerProps {
    t: (key: string) => string;
    theme: string;
}

interface TagInfo {
    name: string;
    count: number;
}

export default function TagManager({ t, theme }: TagManagerProps) {
    const TAG_MANAGER_VIEW_MODE_KEY = "tiez_tag_manager_view_mode";
    const [tags, setTags] = useState<TagInfo[]>([]);
    const [tagSearch, setTagSearch] = useState('');
    const [selectedTag, setSelectedTag] = useState<string | null>(null);
    const [tagItems, setTagItems] = useState<ClipboardEntry[]>([]);
    const [tagColors, setTagColors] = useState<Record<string, string>>({});
    const [editingTag, setEditingTag] = useState<string | null>(null);
    const [newTagName, setNewTagName] = useState('');
    const [loading, setLoading] = useState(false);
    const [viewMode, setViewMode] = useState<'list' | 'grid'>(() => {
        try {
            const saved = window.localStorage.getItem(TAG_MANAGER_VIEW_MODE_KEY);
            return saved === 'list' ? 'list' : 'grid';
        } catch {
            return 'grid';
        }
    });
    const [isDeleting, setIsDeleting] = useState(false);
    const [deleteConfirmation, setDeleteConfirmation] = useState<{ show: boolean, tagName: string | null }>({ show: false, tagName: null });
    const [itemDeleteConfirmation, setItemDeleteConfirmation] = useState<{ show: boolean, id: number | null }>({ show: false, id: null });
    const [isCollapsed, setIsCollapsed] = useState(false);
    const [sortBy, setSortBy] = useState<'time' | 'count'>('time');
    const [isCreatingItem, setIsCreatingItem] = useState(false);
    const [editingItem, setEditingItem] = useState<{ id: number, content: string } | null>(null);
    const [editingNoteId, setEditingNoteId] = useState<number | null>(null);
    const [noteDraft, setNoteDraft] = useState('');
    const noteInputRef = useRef<HTMLTextAreaElement | null>(null);
    const ignoreNoteBlurRef = useRef(false);
    const [newItemContent, setNewItemContent] = useState('');
    const [sidebarWidth, setSidebarWidth] = useState(168);
    const [sidebarHeight, setSidebarHeight] = useState(180);
    const sidebarSizeRef = useRef({ width: 168, height: 180 });
    const [isResizing, setIsResizing] = useState(false);
    const [isStacked, setIsStacked] = useState(false);
    const [isManageMode, setIsManageMode] = useState(false);
    const [selectedItemIds, setSelectedItemIds] = useState<Set<number>>(new Set());
    const containerRef = useRef<HTMLElement | null>(null);

    const selectedTagRef = useRef<string | null>(null);
    useEffect(() => { selectedTagRef.current = selectedTag; }, [selectedTag]);

    useEffect(() => {
        sidebarSizeRef.current = {
            width: isCollapsed ? 48 : sidebarWidth,
            height: sidebarHeight,
        };
    }, [sidebarWidth, sidebarHeight, isCollapsed]);

    const persistSidebarSize = useCallback(() => {
        const { width, height } = sidebarSizeRef.current;
        invoke('save_setting', {
            key: 'app.tag_manager_size',
            value: JSON.stringify({ width, height }),
        }).catch(console.error);
    }, []);

    useEffect(() => {
        invoke<Record<string, string>>('get_settings')
            .then((settings) => {
                const raw = settings['app.tag_manager_size'];
                if (!raw) return;
                const parsed = JSON.parse(raw) as { width?: number; height?: number };
                if (typeof parsed.width === 'number' && parsed.width >= 48) {
                    setSidebarWidth(parsed.width);
                    if (parsed.width < 110) {
                        setIsCollapsed(true);
                    }
                }
                if (typeof parsed.height === 'number' && parsed.height >= 120) {
                    setSidebarHeight(parsed.height);
                }
            })
            .catch(console.error);
    }, []);

    useEffect(() => {
        try {
            window.localStorage.setItem(TAG_MANAGER_VIEW_MODE_KEY, viewMode);
        } catch {
            // Ignore storage write failures and keep UI functional.
        }
    }, [viewMode]);

    useEffect(() => {
        let unlisteners: (() => void)[] = [];
        const setupListeners = async () => {
            const handleUpdate = () => {
                // Don't refresh if we're in the middle of a delete operation
                if (isDeleting) return;
                fetchTags();
                if (selectedTagRef.current) loadTagItems(selectedTagRef.current);
            };
            unlisteners.push(await listen('clipboard-changed', handleUpdate));
            unlisteners.push(await listen('clipboard-updated', handleUpdate));
            unlisteners.push(await listen('clipboard-removed', handleUpdate));
        };
        setupListeners();
        return () => unlisteners.forEach(f => f());
    }, [isDeleting]);

    useEffect(() => { fetchTags(); }, []);

    useEffect(() => {
        const mediaQuery = window.matchMedia("(max-width: 340px)");
        const updateLayoutMode = () => {
            setIsStacked(mediaQuery.matches);
        };

        updateLayoutMode();
        mediaQuery.addEventListener("change", updateLayoutMode);

        return () => mediaQuery.removeEventListener("change", updateLayoutMode);
    }, []);

    useEffect(() => {
        if (!isResizing) return;

        const handleMouseMove = (event: MouseEvent) => {
            const bounds = containerRef.current?.getBoundingClientRect();
            if (!bounds) return;
            if (isStacked) {
                const maxHeight = Math.max(140, bounds.height - 180);
                const nextHeight = Math.min(Math.max(event.clientY - bounds.top, 120), maxHeight);
                setSidebarHeight(nextHeight);
                return;
            }

            const dragPos = event.clientX - bounds.left;
            
            // Auto collapse threshold: 110px
            if (dragPos < 110) {
                if (!isCollapsed) setIsCollapsed(true);
                setSidebarWidth(48);
            } else {
                if (isCollapsed) setIsCollapsed(false);
                const nextWidth = Math.min(dragPos, 320);
                setSidebarWidth(nextWidth);
            }
        };

        const handleMouseUp = () => {
            setIsResizing(false);
            document.body.style.cursor = "";
            document.body.style.userSelect = "";
            persistSidebarSize();
        };

        document.body.style.cursor = isStacked ? "row-resize" : "col-resize";
        document.body.style.userSelect = "none";
        window.addEventListener("mousemove", handleMouseMove);
        window.addEventListener("mouseup", handleMouseUp);

        return () => {
            window.removeEventListener("mousemove", handleMouseMove);
            window.removeEventListener("mouseup", handleMouseUp);
            document.body.style.cursor = "";
            document.body.style.userSelect = "";
        };
    }, [isResizing, isStacked, isCollapsed, persistSidebarSize]);

    const fetchTags = async () => {
        try {
            const [tagMap, colors] = await Promise.all([
                invoke<Record<string, number>>('get_all_tags_info'),
                invoke<Record<string, string>>('get_tag_colors')
            ]);

            const tagArray = Object.entries(tagMap).map(([name, count]) => ({ name, count }));
            tagArray.sort((a, b) => b.count - a.count);
            setTags(tagArray);
            setTagColors(colors || {});

            const activeTag = selectedTagRef.current;
            if (tagArray.length === 0) {
                setSelectedTag(null);
                setTagItems([]);
                return;
            }
            if (!activeTag || !tagArray.some(tag => tag.name === activeTag)) {
                loadTagItems(tagArray[0].name);
            }
        } catch (err) { console.error(err); }
    };

    const loadTagItems = async (tagName: string) => {
        setLoading(true);
        setSelectedTag(tagName);
        try {
            const items = await invoke<ClipboardEntry[]>('get_tag_items', { tag: tagName });
            setTagItems(items || []);
        } catch (err) { console.error(err); setTagItems([]); }
        finally { setLoading(false); }
    };

    const createTag = async (rawName: string) => {
        const trimmed = rawName.trim();
        if (!trimmed) return;

        try {
            await invoke('create_new_tag', { tagName: trimmed });
            setNewTagName('');
            setTagSearch('');
            await fetchTags();
            await loadTagItems(trimmed);
        } catch (err) { console.error(err); }
    };

    const handleRenameTag = async (oldName: string) => {
        const trimmed = newTagName.trim();
        if (!trimmed || trimmed === oldName) { setEditingTag(null); return; }

        if (oldName === 'sensitive' || oldName === '密码') {
            setEditingTag(null);
            return;
        }

        try {
            await invoke('rename_tag_globally', { oldName, newName: trimmed });
            if (selectedTag === oldName) setSelectedTag(trimmed);
            await fetchTags();
            await loadTagItems(trimmed);
            setEditingTag(null);
            setNewTagName('');
        } catch (err) { console.error(err); }
    };

    const handleDeleteTag = async (tagName: string) => {
        if (tagName === 'sensitive' || tagName === '密码') return;
        setIsDeleting(true);
        try {
            await invoke('delete_tag_from_all', { tagName });
            await emit('clipboard-changed'); // Notify App.tsx to refresh
            await fetchTags();
        } catch (err) { console.error(err); }
        finally {
            setIsDeleting(false);
        }
    };

    const handleAddManualItem = async () => {
        if (!newItemContent.trim() || !selectedTag) return;
        try {
            await invoke('add_manual_item', {
                content: newItemContent,
                contentType: 'text',
                tags: [selectedTag]
            });
            setNewItemContent('');
            setIsCreatingItem(false);
            await loadTagItems(selectedTag);
        } catch (err) { console.error(err); }
    };

    const handleUpdateItemContent = async () => {
        if (!editingItem || !editingItem.content.trim()) return;
        try {
            await invoke('update_item_content', {
                id: editingItem.id,
                newContent: editingItem.content
            });
            setEditingItem(null);
            if (selectedTag) await loadTagItems(selectedTag);
        } catch (err) { console.error(err); }
    };

    const copyToClipboard = async (id: number, content: string, type: string) => {
        try {
            if (document.activeElement instanceof HTMLElement) {
                document.activeElement.blur();
            }
            const invokeContent =
                id !== 0 && (type === "image" || type === "video" || type === "file")
                    ? ""
                    : content;
            await invoke('copy_to_clipboard', {
                content: invokeContent,
                contentType: type,
                paste: true,
                id,
                deleteAfterUse: false
            });
        } catch (err) { console.error(err); }
    };

    const openNoteEditor = (item: ClipboardEntry) => {
        ignoreNoteBlurRef.current = true;
        setEditingNoteId(item.id);
        setNoteDraft(item.note || '');
        requestAnimationFrame(() => {
            noteInputRef.current?.focus();
            const el = noteInputRef.current;
            if (el) {
                const len = el.value.length;
                el.setSelectionRange(len, len);
            }
            ignoreNoteBlurRef.current = false;
        });
    };

    const cancelNoteEditor = () => {
        setEditingNoteId(null);
        setNoteDraft('');
    };

    const saveNote = async (id: number, note: string) => {
        const cleaned = note.trim();
        try {
            await invoke('update_entry_note', { id, note: cleaned });
            setTagItems((prev) =>
                prev.map((entry) => (entry.id === id ? { ...entry, note: cleaned } : entry))
            );
            setEditingNoteId(null);
            setNoteDraft('');
            emit('clipboard-changed');
        } catch (err) {
            console.error(err);
        }
    };

    const handleTagItemMouseDown = (e: ReactMouseEvent, item: ClipboardEntry) => {
        if (e.button !== 0 || isManageMode || isModalOpen || editingNoteId !== null) return;
        const target = e.target as HTMLElement;
        if (target.closest('button, input, textarea, [role="button"], .card-note')) {
            return;
        }
        e.preventDefault();
        void copyToClipboard(item.id, item.content, item.content_type);
    };

    const filteredTags = useMemo(() => {
        return tags.filter(t => t.name.toLowerCase().includes(tagSearch.toLowerCase()));
    }, [tags, tagSearch]);

    const normalizedTagSearch = tagSearch.trim().toLowerCase();
    const canCreateTag = normalizedTagSearch.length > 0
        && !tags.some(tag => tag.name.toLowerCase() === normalizedTagSearch);

    const sortedItems = [...tagItems].sort((a, b) => {
        if (sortBy === 'count') return (b.use_count || 0) - (a.use_count || 0);
        return b.timestamp - a.timestamp;
    });

    const formatItemDate = (timestamp: number) => {
        const date = new Date(timestamp);
        const year = date.getFullYear();
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        return `${year}-${month}-${day}`;
    };

    const isModalOpen =
        deleteConfirmation.show ||
        itemDeleteConfirmation.show ||
        isCreatingItem ||
        editingItem !== null;

    return (
        <div className="tag-manager-page">
            <section
                ref={containerRef}
                className={`advanced-workbench tag-manager-workbench theme-${theme} ${isCollapsed ? "sidebar-collapsed" : ""} ${isStacked ? "stacked-layout" : ""}${isModalOpen ? " tag-manager-modal-open" : ""}`}
                style={{
                    ["--advanced-sidebar-width" as string]: isCollapsed ? "48px" : `${sidebarWidth}px`,
                    ["--advanced-sidebar-height" as string]: `${sidebarHeight}px`
                }}
            >
                <aside className="advanced-sidebar tag-manager-sidebar">
                    {!isCollapsed ? (
                        <div className="advanced-sidebar-search tag-manager-sidebar-search">
                            <div className="tag-manager-search-row">
                                <input
                                    className="search-input advanced-search-input"
                                    placeholder={t("find_or_create")}
                                    value={tagSearch}
                                    onMouseDown={() => invoke("activate_window_focus").catch(console.error)}
                                    onChange={(e) => setTagSearch(e.target.value)}
                                    onKeyDown={async (e) => {
                                        if (e.key === "Enter" && tagSearch.trim()) {
                                            const exactMatch = tags.find((tag) => tag.name.toLowerCase() === normalizedTagSearch);
                                            if (exactMatch) {
                                                loadTagItems(exactMatch.name);
                                            } else {
                                                await createTag(tagSearch);
                                            }
                                        }
                                    }}
                                />
                                <button
                                    type="button"
                                    className="tag-manager-collapse-btn"
                                    title={t("collapse") || "收起"}
                                    onClick={() => setIsCollapsed(true)}
                                >
                                    <ChevronLeft size={14} />
                                </button>
                            </div>
                            {canCreateTag && tagSearch.trim() && (
                                <div className="advanced-search-results">
                                    <button
                                        type="button"
                                        className="advanced-search-result-item"
                                        onClick={() => createTag(tagSearch)}
                                    >
                                        <span className="advanced-search-result-name">
                                            {t("create_tag_hint").replace("{tag}", tagSearch.trim())}
                                        </span>
                                        <span className="advanced-search-result-action">{t("create_btn")}</span>
                                    </button>
                                </div>
                            )}
                        </div>
                    ) : (
                        <div className="tag-manager-collapsed-bar">
                            <button
                                type="button"
                                className="tag-manager-collapse-btn"
                                title={t("open") || "展开"}
                                onClick={() => {
                                    setIsCollapsed(false);
                                    if (sidebarWidth < 110) {
                                        setSidebarWidth(168);
                                    }
                                }}
                            >
                                <ChevronRight size={14} />
                            </button>
                        </div>
                    )}

                    <div className="advanced-target-list tag-manager-target-list custom-scrollbar">
                        {filteredTags.map((tag) => (
                            <div
                                key={tag.name}
                                role="button"
                                tabIndex={0}
                                className={`advanced-target-item tag-manager-target-item ${selectedTag === tag.name ? "active" : ""}`}
                                onClick={() => loadTagItems(tag.name)}
                                onKeyDown={(e) => {
                                    if (e.key === "Enter" || e.key === " ") {
                                        e.preventDefault();
                                        loadTagItems(tag.name);
                                    }
                                }}
                                title={tag.name}
                            >
                                <div className="tag-color-wrapper" onClick={(e) => e.stopPropagation()}>
                                    <div
                                        className="tag-color-dot"
                                        style={{ background: tagColors[tag.name] || getTagColor(tag.name, theme) }}
                                        onClick={() => document.getElementById(`color-picker-${tag.name}`)?.click()}
                                    />
                                    <input
                                        type="color"
                                        id={`color-picker-${tag.name}`}
                                        style={{ display: "none" }}
                                        value={tagColors[tag.name] || "#888888"}
                                        onChange={async (e) => {
                                            const newColor = e.target.value;
                                            setTagColors((prev) => ({ ...prev, [tag.name]: newColor }));
                                            await invoke("set_tag_color", { name: tag.name, color: newColor });
                                            await emit("tag-colors-updated");
                                        }}
                                    />
                                </div>
                                {editingTag === tag.name ? (
                                    <input
                                        className="inline-tag-edit"
                                        value={newTagName}
                                        onMouseDown={() => invoke("activate_window_focus").catch(console.error)}
                                        onChange={(e) => setNewTagName(e.target.value)}
                                        autoFocus
                                        onKeyDown={async (e) => {
                                            if (e.key === "Enter") {
                                                await handleRenameTag(tag.name);
                                            } else if (e.key === "Escape") {
                                                setEditingTag(null);
                                            }
                                        }}
                                        onBlur={() => setEditingTag(null)}
                                        onClick={(e) => e.stopPropagation()}
                                    />
                                ) : (
                                    <>
                                        <span className="advanced-target-meta">
                                            <span className="advanced-target-name">{tag.name}</span>
                                            <span className="advanced-target-sub">
                                                {tag.count} {t("tag_count")}
                                            </span>
                                        </span>
                                        {(tag.name !== "sensitive" && tag.name !== "密码") && (
                                            <span className="tag-target-actions">
                                                <button
                                                    type="button"
                                                    className="tag-target-action-btn"
                                                    title={t("rename")}
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        setEditingTag(tag.name);
                                                        setNewTagName(tag.name);
                                                    }}
                                                >
                                                    <Edit2 size={12} />
                                                </button>
                                                <button
                                                    type="button"
                                                    className="tag-target-action-btn danger"
                                                    title={t("delete")}
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        e.preventDefault();
                                                        setDeleteConfirmation({ show: true, tagName: tag.name });
                                                    }}
                                                >
                                                    <Trash2 size={12} />
                                                </button>
                                            </span>
                                        )}
                                    </>
                                )}
                            </div>
                        ))}
                        {filteredTags.length === 0 && !tagSearch.trim() && (
                            <div className="tag-manager-sidebar-status">{t("no_tags")}</div>
                        )}
                        {!isCollapsed && canCreateTag && filteredTags.length === 0 && (
                            <button
                                type="button"
                                className="advanced-target-item tag-manager-target-item create-hint"
                                onClick={() => createTag(tagSearch)}
                            >
                                <div className="tag-color-dot" style={{ border: "1px dashed currentColor", background: "transparent" }} />
                                <span className="advanced-target-meta">
                                    <span className="advanced-target-name">
                                        {t("create_tag_hint").replace("{tag}", tagSearch.trim())}
                                    </span>
                                </span>
                                <Plus size={12} />
                            </button>
                        )}
                    </div>
                </aside>

                {!isCollapsed && (
                    <div
                        className={`advanced-divider ${isResizing ? "active" : ""}`}
                        onMouseDown={(e) => {
                            e.preventDefault();
                            setIsResizing(true);
                        }}
                    >
                        <span className="advanced-divider-handle" />
                    </div>
                )}

                <div className="advanced-editor tag-manager-editor">
                    <div className="advanced-editor-toolbar">
                        <div className="advanced-editor-heading">
                            <div className="advanced-editor-title">
                                {selectedTag ? `#${selectedTag}` : t("tags")}
                            </div>
                            <div className="advanced-editor-subtitle">
                                {selectedTag
                                    ? `${sortedItems.length} ${t("tag_count")}`
                                    : t("select_tag_to_begin")}
                            </div>
                        </div>

                        <div className="advanced-editor-actions tag-manager-toolbar-actions">
                            {selectedTag && (
                                <>
                                    {isManageMode ? (
                                        <>
                                            <button
                                                type="button"
                                                className="tag-manager-sort-btn"
                                                onClick={() => {
                                                    setIsManageMode(false);
                                                    setSelectedItemIds(new Set());
                                                }}
                                            >
                                                {t("cancel") || "取消"}
                                            </button>
                                            <button
                                                type="button"
                                                className="tag-manager-sort-btn danger"
                                                disabled={selectedItemIds.size === 0}
                                                onClick={() => setItemDeleteConfirmation({ show: true, id: -1 })}
                                            >
                                                <Trash2 size={14} />
                                                <span>{t("delete_selected") || "删除选中"}</span>
                                            </button>
                                            <button
                                                type="button"
                                                className="tag-manager-sort-btn active"
                                                disabled={selectedItemIds.size === 0}
                                                onClick={async () => {
                                                    const selectedItems = tagItems.filter((item) => selectedItemIds.has(item.id));
                                                    if (selectedItems.length > 0) {
                                                        const combinedContent = selectedItems.map((item) => item.content).join("\n");
                                                        await invoke("copy_to_clipboard", {
                                                            content: combinedContent,
                                                            contentType: "text",
                                                            paste: true,
                                                            id: -1,
                                                            deleteAfterUse: false
                                                        });
                                                        setIsManageMode(false);
                                                        setSelectedItemIds(new Set());
                                                    }
                                                }}
                                            >
                                                <Copy size={14} />
                                                <span>{t("copy_selected") || "复制选中"}</span>
                                            </button>
                                        </>
                                    ) : (
                                        <button
                                            type="button"
                                            className={`tag-manager-sort-btn ${isManageMode ? "active" : ""}`}
                                            onClick={() => setIsManageMode(true)}
                                            title={t("manage_items") || "管理条目"}
                                        >
                                            <CheckSquare size={14} />
                                            <span>{t("manage") || "管理"}</span>
                                        </button>
                                    )}
                                </>
                            )}
                            <div className="tag-manager-sort-group">
                                <button
                                    type="button"
                                    className={`tag-manager-sort-btn ${sortBy === "time" ? "active" : ""}`}
                                    title={t("sort_time") || "按时间"}
                                    onClick={() => setSortBy("time")}
                                >
                                    <Clock size={12} />
                                    <span>{t("sort_time") || "时间"}</span>
                                </button>
                                <button
                                    type="button"
                                    className={`tag-manager-sort-btn ${sortBy === "count" ? "active" : ""}`}
                                    title={t("sort_usage") || "按频率"}
                                    onClick={() => setSortBy("count")}
                                >
                                    <MousePointer2 size={12} />
                                    <span>{t("sort_usage") || "频率"}</span>
                                </button>
                            </div>
                            <div className="tag-manager-view-toggle">
                                <button
                                    type="button"
                                    className={`toggle-btn ${viewMode === "list" ? "active" : ""}`}
                                    title={t("list_view")}
                                    onClick={() => setViewMode("list")}
                                >
                                    <List size={14} />
                                </button>
                                <button
                                    type="button"
                                    className={`toggle-btn ${viewMode === "grid" ? "active" : ""}`}
                                    title={t("grid_view")}
                                    onClick={() => setViewMode("grid")}
                                >
                                    <LayoutGrid size={14} />
                                </button>
                            </div>
                        </div>
                    </div>

                    <div
                        className={`advanced-rule-list tag-manager-items custom-scrollbar${
                            loading || sortedItems.length === 0 ? " is-empty" : ""
                        }${isManageMode ? " manage-mode" : ""}`}
                    >
                        {loading ? (
                            <div className="tag-manager-status">{t("processing")}</div>
                        ) : sortedItems.length === 0 ? (
                            <div className="advanced-empty-state">
                                <div className="advanced-empty-title">
                                    {selectedTag ? t("no_items") : t("tags")}
                                </div>
                                <div className="advanced-empty-text">
                                    {selectedTag ? t("select_tag") : t("select_tag_to_begin")}
                                </div>
                            </div>
                        ) : (
                            <div className={`items-${viewMode}`}>
                                {sortedItems.map((item) => (
                                    <div
                                        key={item.id}
                                        className={`tag-manager-card ${selectedItemIds.has(item.id) ? "selected" : ""}`}
                                        data-tag-item-id={String(item.id)}
                                        data-tag-item-type={item.content_type}
                                        onMouseDownCapture={(e) => handleTagItemMouseDown(e, item)}
                                        onClick={() => {
                                            if (isModalOpen) return;
                                            if (isManageMode) {
                                                setSelectedItemIds((prev) => {
                                                    const next = new Set(prev);
                                                    if (next.has(item.id)) next.delete(item.id);
                                                    else next.add(item.id);
                                                    return next;
                                                });
                                            }
                                        }}
                                    >
                                        <div className="card-top-row">
                                            <div className="card-actions-left">
                                                {isManageMode ? (
                                                    <div className={`selection-indicator ${selectedItemIds.has(item.id) ? "checked" : ""}`}>
                                                        <div className="inner-check" />
                                                    </div>
                                                ) : (
                                                    <>
                                                        {(item.content_type === "text" || item.content_type === "code") && (
                                                            <button
                                                                className="card-action-btn"
                                                                title="编辑"
                                                                onClick={(e) => {
                                                                    e.stopPropagation();
                                                                    setEditingItem({ id: item.id, content: item.content });
                                                                }}
                                                            >
                                                                <Edit2 size={10} />
                                                            </button>
                                                        )}
                                                        <button
                                                            className={`card-action-btn${item.note || editingNoteId === item.id ? " active" : ""}`}
                                                            title={t("edit_note") || "Note"}
                                                            onClick={(e) => {
                                                                e.stopPropagation();
                                                                if (editingNoteId === item.id) {
                                                                    void saveNote(item.id, noteDraft);
                                                                } else {
                                                                    openNoteEditor(item);
                                                                }
                                                            }}
                                                        >
                                                            <StickyNote size={10} />
                                                        </button>
                                                        <button
                                                            className="card-action-btn"
                                                            onClick={(e) => {
                                                                e.stopPropagation();
                                                                invoke("open_content", {
                                                                    id: item.id,
                                                                    content: item.content,
                                                                    contentType: item.content_type
                                                                });
                                                            }}
                                                            title={t("open")}
                                                        >
                                                            <ExternalLink size={10} />
                                                        </button>
                                                    </>
                                                )}
                                            </div>
                                            {!isManageMode && (
                                                <button
                                                    className="del-btn"
                                                    title="删除"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        setItemDeleteConfirmation({ show: true, id: item.id });
                                                    }}
                                                >
                                                    <X size={10} />
                                                </button>
                                            )}
                                        </div>

                                        {item.content_type === "image" ? (
                                            <div className="card-media">
                                                <img
                                                    src={
                                                        withImageCacheBust(
                                                            item.content.startsWith("data:")
                                                                ? item.content
                                                                : (toTauriLocalImageSrc(item.content) || convertFileSrc(item.content)),
                                                            item.timestamp
                                                        ) || ""
                                                    }
                                                    alt=""
                                                    className="image-preview"
                                                    loading="lazy"
                                                />
                                            </div>
                                        ) : (
                                            <div className="card-body-text">{item.preview || item.content}</div>
                                        )}

                                        {(editingNoteId === item.id || !!item.note) && (
                                            <div
                                                className={`card-note${editingNoteId === item.id ? " editing" : ""}`}
                                                onMouseDown={(e) => e.stopPropagation()}
                                                onClick={(e) => e.stopPropagation()}
                                            >
                                                {editingNoteId === item.id ? (
                                                    <textarea
                                                        ref={noteInputRef}
                                                        className="card-note-input"
                                                        value={noteDraft}
                                                        placeholder={t("note_placeholder") || "Add a note…"}
                                                        rows={2}
                                                        onMouseDown={(e) => {
                                                            e.stopPropagation();
                                                            invoke("activate_window_focus").catch(console.error);
                                                        }}
                                                        onChange={(e) => setNoteDraft(e.target.value)}
                                                        onKeyDown={(e) => {
                                                            if (e.key === "Escape") {
                                                                e.preventDefault();
                                                                e.stopPropagation();
                                                                cancelNoteEditor();
                                                                return;
                                                            }
                                                            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                                                                e.preventDefault();
                                                                e.stopPropagation();
                                                                void saveNote(item.id, noteDraft);
                                                            }
                                                        }}
                                                        onBlur={() => {
                                                            if (ignoreNoteBlurRef.current) return;
                                                            window.setTimeout(() => {
                                                                if (ignoreNoteBlurRef.current) return;
                                                                const active = document.activeElement;
                                                                if (active && noteInputRef.current?.closest(".card-note")?.contains(active)) {
                                                                    return;
                                                                }
                                                                void saveNote(item.id, noteInputRef.current?.value ?? noteDraft);
                                                            }, 0);
                                                        }}
                                                    />
                                                ) : (
                                                    <button
                                                        type="button"
                                                        className="card-note-text"
                                                        title={item.note}
                                                        onClick={(e) => {
                                                            e.stopPropagation();
                                                            openNoteEditor(item);
                                                        }}
                                                    >
                                                        <StickyNote size={11} className="card-note-icon" aria-hidden />
                                                        <span>{item.note}</span>
                                                    </button>
                                                )}
                                            </div>
                                        )}

                                        <div className="card-divider" />
                                        <div className="card-footer">
                                            <span className="meta-time">{formatItemDate(item.timestamp)}</span>
                                            <div className="meta-usage">
                                                <MousePointer2 size={8} /> {item.use_count || 0}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>
                    {selectedTag && !isManageMode && (
                        <button
                            type="button"
                            className="btn-icon tag-manager-fab-add-btn"
                            onClick={() => setIsCreatingItem(true)}
                            title={t("add_item")}
                        >
                            <Plus size={18} />
                        </button>
                    )}
                </div>
            </section>

            <AppModal
                open={deleteConfirmation.show}
                onClose={() => setDeleteConfirmation({ show: false, tagName: null })}
                theme={theme}
                panelClassName="modal-panel confirm-dialog"
            >
                <h3 className="confirm-dialog-title">{t("confirm_delete")}</h3>
                <p className="confirm-dialog-message">
                    {t("confirm_delete_tag")}
                    <br />
                    <span className="tag-highlight" style={{ marginTop: "8px", display: "inline-block" }}>
                        {deleteConfirmation.tagName}
                    </span>
                </p>
                <div className="confirm-dialog-buttons">
                    <button type="button" className="modal-button" onClick={() => setDeleteConfirmation({ show: false, tagName: null })}>
                        {t("cancel")}
                    </button>
                    <button
                        type="button"
                        className="modal-button primary"
                        onClick={() => {
                            if (deleteConfirmation.tagName) {
                                handleDeleteTag(deleteConfirmation.tagName);
                            }
                            setDeleteConfirmation({ show: false, tagName: null });
                        }}
                    >
                        {t("delete")}
                    </button>
                </div>
            </AppModal>

            <AppModal
                open={itemDeleteConfirmation.show}
                onClose={() => setItemDeleteConfirmation({ show: false, id: null })}
                theme={theme}
                panelClassName="modal-panel confirm-dialog"
            >
                <h3 className="confirm-dialog-title">{t("confirm_delete")}</h3>
                <p className="confirm-dialog-message">{t("confirm_delete_desc") || "确定要删除这条记录吗？"}</p>
                <div className="confirm-dialog-buttons">
                    <button type="button" className="modal-button" onClick={() => setItemDeleteConfirmation({ show: false, id: null })}>
                        {t("cancel")}
                    </button>
                    <button
                        type="button"
                        className="modal-button primary"
                        onClick={async () => {
                            if (itemDeleteConfirmation.id === -1) {
                                try {
                                    for (const id of Array.from(selectedItemIds)) {
                                        await invoke("delete_clipboard_entry", { id });
                                    }
                                    setIsManageMode(false);
                                    setSelectedItemIds(new Set());
                                    if (selectedTag) await loadTagItems(selectedTag);
                                    emit("clipboard-changed");
                                } catch (err) {
                                    console.error(err);
                                }
                            } else if (itemDeleteConfirmation.id) {
                                await invoke("delete_clipboard_entry", { id: itemDeleteConfirmation.id });
                                loadTagItems(selectedTag!);
                                emit("clipboard-changed");
                            }
                            setItemDeleteConfirmation({ show: false, id: null });
                        }}
                    >
                        {t("delete")}
                    </button>
                </div>
            </AppModal>

            <AppModal
                open={isCreatingItem}
                onClose={() => setIsCreatingItem(false)}
                theme={theme}
                panelClassName="modal-panel confirm-dialog"
            >
                <h3 className="confirm-dialog-title">{t("add_item")}</h3>
                <textarea
                    className="modal-textarea"
                    value={newItemContent}
                    onChange={(e) => setNewItemContent(e.target.value)}
                    placeholder={t("input_content_placeholder")}
                    autoFocus
                />
                <div className="confirm-dialog-buttons">
                    <button type="button" className="modal-button" onClick={() => setIsCreatingItem(false)}>
                        {t("cancel")}
                    </button>
                    <button type="button" className="modal-button primary" onClick={handleAddManualItem}>
                        {t("confirm")}
                    </button>
                </div>
            </AppModal>

            <AppModal
                open={Boolean(editingItem)}
                onClose={() => setEditingItem(null)}
                theme={theme}
                panelClassName="modal-panel confirm-dialog"
            >
                {editingItem && (
                    <>
                        <h3 className="confirm-dialog-title">{t("edit_item")}</h3>
                        <textarea
                            className="modal-textarea"
                            value={editingItem.content}
                            onChange={(e) => setEditingItem({ ...editingItem, content: e.target.value })}
                            autoFocus
                        />
                        <div className="confirm-dialog-buttons">
                            <button type="button" className="modal-button" onClick={() => setEditingItem(null)}>
                                {t("cancel")}
                            </button>
                            <button type="button" className="modal-button primary" onClick={handleUpdateItemContent}>
                                {t("save")}
                            </button>
                        </div>
                    </>
                )}
            </AppModal>
        </div>
    );
}