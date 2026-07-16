import type { ComponentProps, RefObject, ReactNode } from "react";
import { motion, Reorder, useDragControls } from "framer-motion";
import type { DragControls } from "framer-motion";
import { ArrowUp, Clipboard } from "lucide-react";
import FileTransferChatView from "../../file-transfer/components/FileTransferChatView";
import SettingsPanel from "../../settings/components/SettingsPanel";
import TagManager from "../../tag/components/TagManager";
import EmojiPanel from "../../emoji/components/EmojiPanel";
import { VirtualClipboardList } from "../../clipboard/components/VirtualClipboardList";
import type { ClipboardEntry } from "../../../shared/types";
import type { VirtualClipboardListHandle } from "../../clipboard/types";

type SettingsPanelProps = ComponentProps<typeof SettingsPanel>;
type RenderItem = (
  item: ClipboardEntry,
  index: number,
  dragControls?: DragControls,
  disableLayout?: boolean
) => ReactNode;

interface AppMainContentProps {
  t: (key: string) => string;
  theme: string;
  showSettings: boolean;
  showTagManager: boolean;
  tagManagerEnabled: boolean;
  showEmojiPanel: boolean;
  chatMode: boolean;
  localIp: string;
  actualPort: string;
  settingsPanelProps: SettingsPanelProps;
  emojiFavorites: string[];
  setEmojiFavorites: (val: string[] | ((prev: string[]) => string[])) => void;
  emojiPanelTab: "emoji" | "favorites";
  setEmojiPanelTab: (val: "emoji" | "favorites") => void;
  saveSetting: (key: string, val: string) => void;
  filteredHistory: ClipboardEntry[];
  search: string;
  pinnedItems: ClipboardEntry[];
  unpinnedItems: ClipboardEntry[];
  compactMode: boolean;
  selectedIndex: number;
  isKeyboardMode: boolean;
  virtualListRef: RefObject<VirtualClipboardListHandle | null>;
  pinnedOrderIds: number[];
  isDraggingPinned: boolean;
  handlePinnedIdsReorder: (newOrderIds: number[]) => void;
  handlePinnedDragStart: () => void;
  handlePinnedDragEnd: () => void;
  renderItemContent: RenderItem;
  loadMoreHistory: () => void;
  handleListScroll: (offset: number) => void;
  hasMore: boolean;
  isLoadingMore: boolean;
  showScrollTop: boolean;
  onScrollTop: () => void;
}

const SortableItem = ({
  item,
  index,
  renderItem,
  isFirst,
  onDragStart,
  onDragEnd,
  compactMode
}: {
  item: ClipboardEntry;
  index: number;
  renderItem: RenderItem;
  isFirst?: boolean;
  onDragStart?: () => void;
  onDragEnd?: () => void;
  compactMode: boolean;
}) => {
  const controls = useDragControls();
  return (
    <Reorder.Item
      value={item.id}
      dragListener={false}
      dragControls={controls}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      className={isFirst ? "first-virtual-item" : undefined}
      style={{
        listStyle: "none",
        overflow: "visible",
        paddingTop: isFirst ? "4px" : undefined
      }}
    >
      <div style={{ paddingBottom: compactMode ? "2px" : "4px" }}>
        {renderItem(item, index, controls, true)}
      </div>
    </Reorder.Item>
  );
};

const AppMainContent = ({
  t,
  theme,
  showSettings,
  showTagManager,
  tagManagerEnabled,
  showEmojiPanel,
  chatMode,
  localIp,
  actualPort,
  settingsPanelProps,
  emojiFavorites,
  setEmojiFavorites,
  emojiPanelTab,
  setEmojiPanelTab,
  saveSetting,
  filteredHistory,
  search,
  pinnedItems,
  unpinnedItems,
  compactMode,
  selectedIndex,
  isKeyboardMode,
  virtualListRef,
  pinnedOrderIds,
  isDraggingPinned,
  handlePinnedIdsReorder,
  handlePinnedDragStart,
  handlePinnedDragEnd,
  renderItemContent,
  loadMoreHistory,
  handleListScroll,
  hasMore,
  isLoadingMore,
  showScrollTop,
  onScrollTop
}: AppMainContentProps) => {
  const orderedPinnedIds = pinnedOrderIds;

  if (showTagManager && tagManagerEnabled) {
    return (
      <motion.div
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        style={{ height: "100%", overflow: "hidden", boxSizing: "border-box" }}
      >
        <TagManager t={t} theme={theme} />
      </motion.div>
    );
  }

  if (showEmojiPanel) {
    return (
      <motion.div
        initial={{ opacity: 0, x: 20 }}
        animate={{ opacity: 1, x: 0 }}
        style={{ height: "100%", overflow: "hidden" }}
      >
        <EmojiPanel
          t={t}
          favorites={emojiFavorites}
          setFavorites={setEmojiFavorites}
          activeTab={emojiPanelTab}
          setActiveTab={setEmojiPanelTab}
          saveSetting={saveSetting}
        />
      </motion.div>
    );
  }

  if (showSettings) {
    if (chatMode) {
      return (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          style={{ height: "100%", overflow: "hidden" }}
        >
          <FileTransferChatView t={t} localIp={localIp} actualPort={actualPort} />
        </motion.div>
      );
    }

    return (
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.16 }}
        className="settings-view"
        style={{
          maxWidth: settingsPanelProps.settingsSubpage === "advanced" ? "min(1120px, 100%)" : undefined
        }}
      >
        <SettingsPanel {...settingsPanelProps} />
      </motion.div>
    );
  }

  if (filteredHistory.length === 0) {
    return (
      <div className="empty-state">
        <Clipboard size={40} opacity={0.2} style={{ marginBottom: "12px" }} />
        {search ? (
          <p>{t("no_records")}</p>
        ) : (
          <>
            <p
              style={{
                fontSize: "15px",
                fontWeight: "bold",
                color: "var(--text-primary)",
                marginBottom: "4px"
              }}
            >
              {t("empty_title")}
            </p>
            <p style={{ fontSize: "12px", opacity: 0.6 }}>{t("empty_desc")}</p>
          </>
        )}
      </div>
    );
  }

  return (
    <>
      {filteredHistory.length > 0 && (
        <div className="history-list-container">
          <VirtualClipboardList
            ref={virtualListRef}
            items={unpinnedItems}
            compactMode={compactMode}
            selectedIndex={selectedIndex - pinnedItems.length}
            isKeyboardMode={isKeyboardMode}
            header={
              pinnedItems.length > 0 ? (
                <Reorder.Group
                  axis="y"
                  values={orderedPinnedIds}
                  onReorder={handlePinnedIdsReorder}
                  className={isDraggingPinned ? "pinned-reorder dragging" : "pinned-reorder"}
                  style={{ listStyle: "none", padding: 0 }}
                >
                  {pinnedItems.map((item, index) => (
                    <SortableItem
                      key={item.id}
                      item={item}
                      index={index}
                      renderItem={renderItemContent}
                      isFirst={index === 0}
                      onDragStart={handlePinnedDragStart}
                      onDragEnd={handlePinnedDragEnd}
                      compactMode={compactMode}
                    />
                  ))}
                </Reorder.Group>
              ) : null
            }
            renderItem={(item, index, isFirst?: boolean) => {
              const el = renderItemContent(item, pinnedItems.length + index, undefined, true);
              if (isFirst && pinnedItems.length === 0) {
                return (
                  <div className="first-virtual-item" style={{ height: "100%", paddingTop: "4px" }}>
                    {el}
                  </div>
                );
              }
              return el;
            }}
            onLoadMore={loadMoreHistory}
            onScroll={handleListScroll}
            hasMore={hasMore}
            isLoading={isLoadingMore}
          />
          {showScrollTop && (
            <button
              type="button"
              className="btn-icon scroll-top-button"
              onClick={onScrollTop}
              aria-label={t("scroll_to_top")}
              title={t("scroll_to_top")}
            >
              <ArrowUp size={16} />
            </button>
          )}
        </div>
      )}
    </>
  );
};

export default AppMainContent;
