<template>
  <router-view />
</template>

<script setup lang="ts">
</script>

<style>
html, body, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
}

/* 搜索栏表单控件统一宽度 */
.search-card .el-input {
  width: 200px;
}
.search-card .el-select {
  width: 180px;
}
</style>
