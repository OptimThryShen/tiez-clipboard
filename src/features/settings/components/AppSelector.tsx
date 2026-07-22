import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown } from "lucide-react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { InstalledAppOption } from "../../app/types";

interface AppSelectorProps {
    type: string | null;
    installedApps: InstalledAppOption[];
    onSelect: (val: string) => void;
    theme: string;
    t: (key: string) => string;
    colorMode: string;
}

interface FlattenedAppItem {
    group: "recommended" | "all";
    app: InstalledAppOption;
}

const AppSelector = ({
    type,
    installedApps,
    onSelect,
    t
}: AppSelectorProps) => {
    const [recommended, setRecommended] = useState<InstalledAppOption[]>([]);
    const [loading, setLoading] = useState(false);
    const [query, setQuery] = useState("");
    const [isOpen, setIsOpen] = useState(false);
    const [selectedIndex, setSelectedIndex] = useState(0);
    const rootRef = useRef<HTMLDivElement | null>(null);
    const inputRef = useRef<HTMLInputElement | null>(null);
    const itemRefs = useRef<Array<HTMLButtonElement | null>>([]);

    useEffect(() => {
        if (!type) {
            setRecommended([]);
            return;
        }

        const fetchRecommended = async () => {
            setLoading(true);
            try {
                let ext = "";
                let keywords: string[] = [];

                switch (type) {
                    case "image":
                        ext = ".png";
                        keywords = ["photo", "paint", "image", "adobe", "picture", "snip", "viewer", "画图", "照片", "看图"];
                        break;
                    case "text":
                    case "code":
                        ext = ".txt";
                        keywords = ["text", "note", "code", "edit", "write", "office", "word", "记事本", "文档"];
                        break;
                    case "html":
                    case "link":
                    case "url":
                        ext = ".html";
                        keywords = ["browser", "chrome", "edge", "firefox", "web", "internet"];
                        break;
                    case "rtf":
                        ext = ".rtf";
                        keywords = ["word", "office", "write"];
                        break;
                    case "rich_text":
                        ext = ".html";
                        keywords = ["word", "office", "write", "notes", "browser", "chrome", "edge", "firefox", "wps"];
                        break;
                    case "file":
                        ext = ".txt";
                        break;
                    default:
                        ext = "";
                }

                let recApps: InstalledAppOption[] = [];
                if (ext) {
                    try {
                        const rec = await invoke<{ name: string; path: string }[]>("get_associated_apps", { extension: ext });
                        recApps = rec.map((app) => ({ label: app.name, value: app.path }));
                    } catch {
                        // Ignore recommendation lookup failures.
                    }
                }

                const localMatches = installedApps.filter((app) => {
                    const lower = app.label.toLowerCase();
                    const isMatch = keywords.some((keyword) => lower.includes(keyword));
                    const alreadyIncluded = recApps.some((existing) => existing.value === app.value);
                    return isMatch && !alreadyIncluded;
                });

                setRecommended([...recApps, ...localMatches]);
            } finally {
                setLoading(false);
            }
        };

        fetchRecommended();
    }, [type, installedApps]);

    useEffect(() => {
        setQuery("");
        setIsOpen(false);
        setSelectedIndex(0);
    }, [type]);

    useEffect(() => {
        const handlePointerDown = (event: MouseEvent) => {
            if (!rootRef.current?.contains(event.target as Node)) {
                setIsOpen(false);
            }
        };

        document.addEventListener("mousedown", handlePointerDown);
        return () => document.removeEventListener("mousedown", handlePointerDown);
    }, []);

    const otherApps = useMemo(() => {
        let others = installedApps.filter((app) => !recommended.some((rec) => rec.value === app.value));

        if (type) {
            others = others.filter((app) => {
                const name = app.label.toLowerCase();
                if (type === "image") {
                    const blocked = ["music", "player", "sound", "video", "audio", "code", "terminal", "powershell", "cmd"];
                    return !blocked.some((keyword) => name.includes(keyword));
                }
                if (type === "audio" || type === "video") {
                    const blocked = ["photo", "image", "paint", "text", "note", "code", "word", "excel"];
                    return !blocked.some((keyword) => name.includes(keyword));
                }
                return true;
            });
        }

        return others;
    }, [installedApps, recommended, type]);

    const normalizedQuery = query.trim().toLowerCase();

    const matchApp = (app: InstalledAppOption) => {
        if (!normalizedQuery) return true;
        return app.label.toLowerCase().includes(normalizedQuery) || app.value.toLowerCase().includes(normalizedQuery);
    };

    const filteredRecommended = useMemo(() => recommended.filter(matchApp), [recommended, normalizedQuery]);
    const filteredOtherApps = useMemo(() => otherApps.filter(matchApp), [otherApps, normalizedQuery]);

    const flattenedItems = useMemo<FlattenedAppItem[]>(
        () => [
            ...filteredRecommended.map((app) => ({ group: "recommended" as const, app })),
            ...filteredOtherApps.map((app) => ({ group: "all" as const, app }))
        ],
        [filteredRecommended, filteredOtherApps]
    );

    useEffect(() => {
        if (flattenedItems.length === 0) {
            setSelectedIndex(0);
            return;
        }
        setSelectedIndex((current) => Math.min(current, flattenedItems.length - 1));
    }, [flattenedItems]);

    useEffect(() => {
        if (!isOpen) return;
        const selectedItem = itemRefs.current[selectedIndex];
        if (selectedItem) {
            selectedItem.scrollIntoView({ block: "nearest" });
        }
    }, [isOpen, selectedIndex]);

    const controlBackground = "var(--bg-input)";
    const menuBackground = "var(--popover-background)";
    const textColor = "var(--text-primary)";
    const subtleText = "var(--text-secondary)";
    const selectedBackground = "var(--menu-item-hover-background)";

    const handleSelect = (app: InstalledAppOption) => {
        setIsOpen(false);
        onSelect(app.value);
    };

    const handleInputKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
        if (event.key === "ArrowDown") {
            event.preventDefault();
            if (!isOpen) {
                setIsOpen(true);
                return;
            }
            if (flattenedItems.length > 0) {
                setSelectedIndex((current) => Math.min(current + 1, flattenedItems.length - 1));
            }
            return;
        }

        if (event.key === "ArrowUp") {
            event.preventDefault();
            if (!isOpen) {
                setIsOpen(true);
                return;
            }
            if (flattenedItems.length > 0) {
                setSelectedIndex((current) => Math.max(current - 1, 0));
            }
            return;
        }

        if (event.key === "Enter") {
            if (!isOpen) {
                setIsOpen(true);
                return;
            }

            const selected = flattenedItems[selectedIndex];
            if (selected) {
                event.preventDefault();
                handleSelect(selected.app);
            }
            return;
        }

        if (event.key === "Escape") {
            setIsOpen(false);
        }
    };

    const renderSection = (
        label: string,
        items: InstalledAppOption[],
        group: "recommended" | "all",
        startIndex: number
    ) => {
        if (items.length === 0) return null;

        return (
            <div style={{ display: "flex", flexDirection: "column" }}>
                <div
                    style={{
                        padding: "8px 12px 6px",
                        fontSize: "11px",
                        fontWeight: 700,
                        color: subtleText,
                        textTransform: "uppercase",
                        letterSpacing: "0.04em"
                    }}
                >
                    {label}
                </div>
                {items.map((app, offset) => {
                    const index = startIndex + offset;
                    const selected = index === selectedIndex;
                    return (
                        <button
                            key={`${group}-${app.value}`}
                            ref={(node) => {
                                itemRefs.current[index] = node;
                            }}
                            type="button"
                            title={app.value}
                            onMouseEnter={() => setSelectedIndex(index)}
                            onMouseDown={(event) => {
                                event.preventDefault();
                                handleSelect(app);
                            }}
                            style={{
                                border: "none",
                                background: selected ? selectedBackground : "transparent",
                                color: selected ? "var(--menu-item-hover-color)" : textColor,
                                textAlign: "left",
                                padding: "10px 12px",
                                cursor: "pointer",
                                fontSize: "13px",
                                fontWeight: 500,
                                lineHeight: 1.3,
                                width: "100%"
                            }}
                        >
                            {app.label}
                        </button>
                    );
                })}
            </div>
        );
    };

    return (
        <div
            ref={rootRef}
            className="app-selector"
            style={{
                position: "relative",
                width: "100%"
            }}
        >
            <div
                style={{
                    position: "relative",
                    width: "100%"
                }}
            >
                <input
                    ref={inputRef}
                    autoFocus
                    value={query}
                    onChange={(event) => {
                        setQuery(event.target.value);
                        setSelectedIndex(0);
                        if (!isOpen) setIsOpen(true);
                    }}
                    onFocus={() => {
                        setIsOpen(true);
                    }}
                    onClick={() => setIsOpen(true)}
                    onKeyDown={handleInputKeyDown}
                    placeholder={loading ? t("searching_apps") : t("search_apps_placeholder")}
                    spellCheck={false}
                    style={{
                        width: "100%",
                        minHeight: "36px",
                        padding: "0 36px 0 12px",
                        borderRadius: "var(--input-radius)",
                        border: "var(--input-border)",
                        background: controlBackground,
                        color: textColor,
                        outline: "none",
                        boxShadow: "none",
                        fontSize: "13px"
                    }}
                />
                <button
                    type="button"
                    aria-label={t("select_app_title")}
                    onMouseDown={(event) => {
                        event.preventDefault();
                        setIsOpen((open) => !open);
                        inputRef.current?.focus();
                    }}
                    style={{
                        position: "absolute",
                        top: "50%",
                        right: "8px",
                        transform: "translateY(-50%)",
                        border: "none",
                        background: "transparent",
                        color: subtleText,
                        cursor: "pointer",
                        display: "inline-flex",
                        alignItems: "center",
                        justifyContent: "center",
                        padding: 0
                    }}
                >
                    <ChevronDown
                        size={16}
                        style={{
                            transform: isOpen ? "rotate(180deg)" : "rotate(0deg)",
                            transition: "transform 0.15s ease"
                        }}
                    />
                </button>
            </div>

            {isOpen && (
                <div
                    style={{
                        position: "absolute",
                        top: "calc(100% + 6px)",
                        left: 0,
                        right: 0,
                        zIndex: 20,
                        maxHeight: "260px",
                        overflowY: "auto",
                        background: menuBackground,
                        color: "var(--popover-color)",
                        border: "var(--popover-border)",
                        borderRadius: "var(--popover-radius)",
                        boxShadow: "var(--popover-shadow)",
                        backdropFilter: "var(--popover-backdrop-filter)"
                    }}
                >
                    {loading ? (
                        <div style={{ padding: "12px", fontSize: "13px", color: subtleText }}>
                            {t("searching_apps")}
                        </div>
                    ) : flattenedItems.length === 0 ? (
                        <div style={{ padding: "12px", fontSize: "13px", color: subtleText }}>
                            {t("no_matching_apps")}
                        </div>
                    ) : (
                        <>
                            {renderSection(t("system_recommended"), filteredRecommended, "recommended", 0)}
                            {renderSection(t("all_apps"), filteredOtherApps, "all", filteredRecommended.length)}
                        </>
                    )}
                </div>
            )}
        </div>
    );
};

export default AppSelector;
