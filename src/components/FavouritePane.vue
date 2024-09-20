<script setup lang="ts">

import ComicCard from "./ComicCard.vue";
import {computed, ref, watch} from "vue";
import {ComicInfo} from "../types.ts";
import {commands, Pagination, Sort} from "../bindings.ts";
import {useNotification} from "naive-ui";

const notification = useNotification();

const props = defineProps<{
  searchById: (comicId: string) => void;
  currentTabName: "search" | "episode" | "favourite";
}>();

const comicSimplePagination = ref<Pagination<ComicInfo>>();
const sortSelected = ref<Sort>("TimeNewest");// TODO: 添加一个选择器来控制这个值

const comicInfoPagination = computed<Pagination<ComicInfo> | undefined>(() => {
  const pagination = comicSimplePagination.value;
  if (pagination === undefined) {
    return undefined;
  }
  return {
    ...pagination,
    docs: pagination.docs.map(({_id, title, author, categories, thumb}) => ({_id, title, author, categories, thumb,})),
  };
});

async function getFavourite(sort: Sort, page: number) {
  const result = await commands.getFavouriteComics(sort, page);
  if (result.status === "error") {
    notification.error({title: "获取收藏失败", description: result.error});
    return;
  }
  comicSimplePagination.value = result.data;
}

watch(() => props.currentTabName, async () => {
  if (comicSimplePagination.value !== undefined || props.currentTabName !== "favourite") {
    return;
  }
  await getFavourite(sortSelected.value, 1);
}, {immediate: true});

</script>

<template>
  <div v-if="comicInfoPagination!==undefined" class="flex flex-col gap-row-1 overflow-auto p-2">
    <comic-card :comic-info-pagination="comicInfoPagination"
                :on-click-item="searchById"/>

    <n-pagination :total="comicInfoPagination.total"
                  :page-count="comicInfoPagination.pages"
                  :page="comicInfoPagination.page"
                  @update:page="getFavourite(sortSelected, $event)"/>
  </div>
</template>
