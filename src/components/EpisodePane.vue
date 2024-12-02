<script setup lang="ts">
import {SelectionArea, SelectionEvent, SelectionOptions} from "@viselect/vue";
import {nextTick, ref, watch} from "vue";
import {Comic, commands} from "../bindings.ts";
import {useNotification} from "naive-ui";

const notification = useNotification();

const pickedComic = defineModel<Comic | undefined>("pickedComic", {required: true});

defineProps<{
  searchById: (comicId: string) => void;
}>();

const dropdownX = ref<number>(0);
const dropdownY = ref<number>(0);
const showDropdown = ref<boolean>(false);
const dropdownOptions = [
  {label: "勾选", key: "check"},
  {label: "取消勾选", key: "uncheck"},
  {label: "全选", key: "check all"},
  {label: "取消全选", key: "uncheck all"},
];
const checkedIds = ref<string[]>([]);
const selectedIds = ref<Set<string>>(new Set());
//记录这次框选是否改动了选中的元素
const selectedChanged = ref<boolean>(false);
const selectionAreaRef = ref<InstanceType<typeof SelectionArea>>();

watch(pickedComic, () => {
  checkedIds.value = [];
  selectedIds.value.clear();
  selectionAreaRef.value?.selection?.clearSelection();
});

watch(selectedIds.value, () => {
  selectedChanged.value = true;
});

function extractIds(elements: Element[]): string[] {
  return elements.map(element => element.getAttribute("data-key"))
      .filter(Boolean)
      .filter(id => id !== null)
      .filter(id => {
        const ep = pickedComic.value?.episodes.find(ep => ep.epId === id);
        if (ep === undefined) {
          return false;
        }
        return !ep.isDownloaded;
      });
}

function onMouseDown(event: MouseEvent) {
  if (event.ctrlKey || event.metaKey) {
    return;
  }
  if (event?.button === 0) {
    selectedChanged.value = false;
  }
}

function onMouseUp(event: MouseEvent) {
  // 如果是左键点击，且没有改动选中的元素，则清空选中
  if (event?.button === 0 && !selectedChanged.value) {
    selectedIds.value.clear();
    selectionAreaRef.value?.selection?.clearSelection();
  }
}

function onDragStart({event, selection}: SelectionEvent) {
  if (!event?.ctrlKey && !event?.metaKey) {
    selection.clearSelection();
    selectedIds.value.clear();
  }
}

function onDragMove({store: {changed: {added, removed}}}: SelectionEvent) {
  extractIds(added).forEach(id => selectedIds.value.add(id));
  extractIds(removed).forEach(id => selectedIds.value.delete(id));
}

function onDropdownSelect(key: "check" | "uncheck" | "check all" | "uncheck all") {
  showDropdown.value = false;
  if (key === "check") {
    // 只有未勾选的才会被勾选
    [...selectedIds.value]
        .filter(id => !checkedIds.value.includes(id))
        .forEach(id => checkedIds.value.push(id));
  } else if (key === "uncheck") {
    checkedIds.value = checkedIds.value.filter(id => !selectedIds.value.has(id));
  } else if (key === "check all") {
    // 只有未锁定的才会被勾选
    pickedComic.value?.episodes
        .filter(ep => !ep.isDownloaded && !checkedIds.value.includes(ep.epId))
        .forEach(ep => checkedIds.value.push(ep.epId));
  } else if (key === "uncheck all") {
    checkedIds.value.length = 0;
  }
}

async function onContextMenu(e: MouseEvent) {
  showDropdown.value = false;
  await nextTick();
  showDropdown.value = true;
  dropdownX.value = e.clientX;
  dropdownY.value = e.clientY;
}

async function downloadEpisodes() {
  const episodesToDownload = pickedComic.value?.episodes.filter(ep => !ep.isDownloaded && checkedIds.value.includes(ep.epId));
  if (episodesToDownload === undefined) {
    return;
  }
  await commands.downloadEpisodes(episodesToDownload);

  for (const downloadedEp of episodesToDownload) {
    const episode = pickedComic.value?.episodes.find(ep => ep.epId === downloadedEp.epId);
    if (episode !== undefined) {
      episode.isDownloaded = true;
      checkedIds.value = checkedIds.value.filter(id => id !== downloadedEp.epId);
    }
  }
}

async function refreshEpisodes() {
  if (pickedComic.value === undefined) {
    return;
  }
  const result = await commands.getComic(pickedComic.value._id.trim());
  if (result.status === "error") {
    notification.error({title: "刷新失败(获取章节详情失败)", description: result.error});
    return;
  }
  pickedComic.value = result.data;
}

</script>

<template>
  <div class="h-full flex flex-col gap-row-1">
    <div class="flex flex-justify-around">
      <span>总章数：{{ pickedComic?.episodes.length }}</span>
      <n-divider vertical></n-divider>
      <span>已下载：{{ pickedComic?.episodes.filter(ep => ep.isDownloaded).length }}</span>
      <n-divider vertical></n-divider>
      <span>已勾选：{{ checkedIds.length }}</span>
    </div>
    <div class="flex justify-between">
      左键拖动进行框选，右键打开菜单
      <n-button size="tiny" :disabled="pickedComic===undefined" @click="refreshEpisodes" class="w-1/6">刷新</n-button>
      <n-button size="tiny"
                :disabled="pickedComic===undefined"
                type="primary"
                @click="downloadEpisodes"
                class="w-1/4">
        下载勾选章节
      </n-button>
    </div>
    <n-empty v-if="pickedComic===undefined" description="请先进行漫画搜索">
    </n-empty>
    <SelectionArea v-else
                   ref="selectionAreaRef"
                   class="selection-container  flex-1"
                   :options="{selectables: '.selectable'} as SelectionOptions"
                   @contextmenu="onContextMenu"
                   @mousedown="onMouseDown"
                   @mouseup="onMouseUp"
                   @move="onDragMove"
                   @start="onDragStart">
      <n-checkbox-group v-model:value="checkedIds" class="grid grid-cols-3 gap-1.5 w-full">
        <n-checkbox v-for="{epId, epTitle, isDownloaded} in pickedComic.episodes"
                    :key="epId"
                    :data-key="epId"
                    class="selectable hover:bg-gray-200!"
                    :value="epId"
                    :label="epTitle"
                    :disabled="isDownloaded"
                    :class="{ selected: selectedIds.has(epId), downloaded: isDownloaded }"/>
      </n-checkbox-group>
    </SelectionArea>

    <div v-if="pickedComic!==undefined" class="flex">
      <img
          class="w-24"
          :src="`${pickedComic.thumb.fileServer}/static/${pickedComic.thumb.path}`"
          alt=""
          referrerpolicy="no-referrer"/>
      <div class="flex flex-col w-full justify-between">
        <div class="flex flex-col">
          <span class="font-bold text-xl line-clamp-2">{{ pickedComic.title }}</span>
          <span class="text-red">作者：{{ pickedComic.author }}</span>
          <span class="text-gray" v-html="`分类：${pickedComic.categories}`"></span>
        </div>
      </div>
    </div>
    <n-dropdown
        placement="bottom-start"
        trigger="manual"
        :x="dropdownX"
        :y="dropdownY"
        :options="dropdownOptions"
        :show="showDropdown"
        :on-clickoutside="()=>showDropdown=false"
        @select="onDropdownSelect"
    />
  </div>
</template>

<style scoped>
.selection-container {
  @apply user-select-none overflow-auto;
}

.selection-container .selected {
  @apply bg-[rgb(204,232,255)];
}

.selection-container .downloaded {
  @apply bg-[rgba(24,160,88,0.16)];
}

:deep(.n-checkbox__label) {
  @apply overflow-hidden whitespace-nowrap text-ellipsis;
}

:global(.selection-area) {
  @apply bg-[rgba(46,115,252,0.5)];
}
</style>