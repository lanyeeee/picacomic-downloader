<script setup lang="ts">
import {ref} from "vue";
import {ComicInSearch, commands, Episode, Pagination, Sort} from "../bindings.ts";
import {useNotification} from "naive-ui";
import SearchResult from "./SearchResult.vue";

const notification = useNotification();

const sortOptions = [
  {label: "新到旧", value: "TimeNewest"},
  {label: "旧到新", value: "TimeOldest"},
  {label: "最多爱心", value: "LikeMost"},
  {label: "最多指名", value: "ViewMost"},
];

const episodes = defineModel<Episode[] | undefined>("episodes", {required: true});
const currentTabName = defineModel<"search" | "episode">("currentTabName", {required: true});

const searchInput = ref<string>("");
const comicIdInput = ref<string>("");
const sortSelected = ref<Sort>("TimeNewest");
const comicInSearchPagination = ref<Pagination<ComicInSearch>>();

async function searchByKeyword(keyword: string, sort: Sort, page: number, categories: string[]) {
  const result = await commands.searchComic(keyword, sort, page, categories);
  if (result.status === "error") {
    notification.error({title: "搜索失败", description: result.error});
    return;
  }
  comicInSearchPagination.value = result.data;
  console.log("comicInSearchPagination", comicInSearchPagination.value);
}

async function searchById(comicId: string) {
  const result = await commands.getEpisodes(comicId);
  if (result.status === "error") {
    notification.error({title: "获取章节详情失败", description: result.error});
    return;
  }
  episodes.value = result.data;
  currentTabName.value = "episode";
}

</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex flex-col">
      <div class="grid grid-cols-[4fr_1fr]">
        <div class="flex">
          <n-input class="text-align-left"
                   size="tiny"
                   v-model:value="searchInput"
                   placeholder=""
                   clearable
                   @keydown.enter="searchByKeyword(searchInput.trim(), sortSelected, 1, [])">
            <template #prefix>
              漫画名:
            </template>
          </n-input>
          <n-button size="tiny" @click="searchByKeyword(searchInput.trim(), sortSelected, 1, [])">搜索</n-button>
        </div>
        <n-select class="flex"
                  v-model:value="sortSelected"
                  :options="sortOptions"
                  :show-checkmark="false"
                  size="tiny"
                  @update-value="searchByKeyword(searchInput.trim(), $event, 1, [])"/>
      </div>
      <div class="flex">
        <n-input class="text-align-left"
                 size="tiny"
                 v-model:value="comicIdInput"
                 placeholder=""
                 clearable
                 @keydown.enter="searchById(comicIdInput.trim())">
          <template #prefix>
            漫画ID:
          </template>
        </n-input>
        <n-button size="tiny" @click="searchById(comicIdInput.trim())">直达</n-button>
      </div>
    </div>

    <div v-if="comicInSearchPagination!==undefined" class="flex flex-col gap-row-1 overflow-auto p-2">
      <search-result :comic-in-search-pagination="comicInSearchPagination"
                     :on-click-item="searchById"/>

      <n-pagination :total="comicInSearchPagination.total"
                    :page-count="comicInSearchPagination.pages"
                    :page="comicInSearchPagination.page"
                    @update:page="searchByKeyword(searchInput.trim(), sortSelected, $event, [])"/>
    </div>


  </div>
</template>