<script setup lang="ts">
import {computed, ref} from "vue";
import {ComicInSearchRespData, commands, Pagination, Sort} from "../bindings.ts";
import {useNotification} from "naive-ui";
import ComicCard from "./ComicCard.vue";
import {ComicInfo} from "../types.ts";

const notification = useNotification();

const sortOptions = [
  {label: "新到旧", value: "TimeNewest"},
  {label: "旧到新", value: "TimeOldest"},
  {label: "最多爱心", value: "LikeMost"},
  {label: "最多指名", value: "ViewMost"},
];

defineProps<{
  searchById: (comicId: string) => void;
}>();

const searchInput = ref<string>("");
const comicIdInput = ref<string>("");
const sortSelected = ref<Sort>("TimeNewest");
const comicInSearchPagination = ref<Pagination<ComicInSearchRespData>>();

const comicInfoPagination = computed<Pagination<ComicInfo> | undefined>(() => {
  const pagination = comicInSearchPagination.value;
  if (pagination === undefined) {
    return undefined;
  }
  return {
    ...pagination,
    docs: pagination.docs.map(({_id, title, author, categories, thumb}) => ({_id, title, author, categories, thumb,})),
  };
});

async function searchByKeyword(keyword: string, sort: Sort, page: number, categories: string[]) {
  const result = await commands.searchComic(keyword, sort, page, categories);
  if (result.status === "error") {
    notification.error({title: "搜索失败", description: result.error});
    return;
  }
  comicInSearchPagination.value = result.data;
}

</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex flex-col gap-row-1 pt-1">
      <div class="grid grid-cols-[5fr_3fr] gap-col-1">
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
        <div class="flex gap-col-1">
          <n-button type="primary"
                    secondary
                    size="tiny"
                    @click="searchByKeyword(searchInput.trim(), sortSelected, 1, [])">
            搜索
          </n-button>
          <n-select class="flex"
                    v-model:value="sortSelected"
                    :options="sortOptions"
                    :show-checkmark="false"
                    size="tiny"
                    @update-value="searchByKeyword(searchInput.trim(), $event, 1, [])"/>
        </div>
      </div>
      <div class="grid grid-cols-[5fr_3fr] gap-col-1">
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
        <n-button type="primary"
                  secondary
                  size="tiny" @click="searchById(comicIdInput.trim())">
          直达
        </n-button>
      </div>
    </div>

    <div v-if="comicInfoPagination!==undefined" class="flex flex-col gap-row-1 overflow-auto p-2">
      <div class="flex flex-col gap-row-2 overflow-auto">
        <comic-card v-for="comicInfo in comicInfoPagination.docs"
                    :key="comicInfo._id"
                    :comic-info="comicInfo"
                    :onClickItem="searchById"/>
      </div>
      <n-pagination :total="comicInfoPagination.total"
                    :page-count="comicInfoPagination.pages"
                    :page="comicInfoPagination.page"
                    @update:page="searchByKeyword(searchInput.trim(), sortSelected, $event, [])"/>
    </div>
  </div>
</template>