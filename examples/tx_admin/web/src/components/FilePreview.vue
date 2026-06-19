<!-- 文件预览组件
     支持浏览器原生格式：图片、PDF、视频、音频、文本/代码
     扩展方式：在 fileCategories 中新增分类 + 对应模板即可

     所有资源通过 axios 带 token 请求为 blob，再用 ObjectURL 渲染，
     避免浏览器直接请求不带 Authorization 导致 401。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="fileName"
    :width="dialogWidth"
    :destroy-on-close="true"
    @update:model-value="$emit('update:visible', $event)"
  >
    <div class="preview-container">
      <!-- 加载中（非文本类统一 loading） -->
      <div v-if="blobLoading" class="preview-loading">
        <el-icon class="is-loading" :size="32"><Loading /></el-icon>
        <span>加载中...</span>
      </div>

      <!-- 图片 -->
      <div v-else-if="category === 'image'" class="preview-image">
        <img :src="blobUrl" :alt="fileName" />
      </div>

      <!-- PDF -->
      <div v-else-if="category === 'pdf'" class="preview-pdf">
        <iframe :src="blobUrl" frameborder="0"></iframe>
      </div>

      <!-- 视频 -->
      <div v-else-if="category === 'video'" class="preview-video">
        <video :src="blobUrl" controls>
          您的浏览器不支持视频播放
        </video>
      </div>

      <!-- 音频 -->
      <div v-else-if="category === 'audio'" class="preview-audio">
        <div class="audio-info">
          <el-icon :size="64"><Headset /></el-icon>
          <span>{{ fileName }}</span>
        </div>
        <audio :src="blobUrl" controls>
          您的浏览器不支持音频播放
        </audio>
      </div>

      <!-- 文本/代码 -->
      <div v-else-if="category === 'text'" class="preview-text">
        <div v-if="textLoading" class="text-loading">
          <el-icon class="is-loading" :size="32"><Loading /></el-icon>
          <span>加载中...</span>
        </div>
        <pre v-else class="text-content">{{ textContent }}</pre>
      </div>

      <!-- 不支持的格式 -->
      <div v-else class="preview-unsupported">
        <el-icon :size="64"><DocumentRemove /></el-icon>
        <p>该文件类型暂不支持预览</p>
        <p class="file-type-hint">{{ fileName }}</p>
      </div>
    </div>

    <template #footer>
      <el-button @click="$emit('update:visible', false)">关闭</el-button>
      <el-button type="primary" @click="handleDownload">下载</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { Headset, Loading, DocumentRemove } from '@element-plus/icons-vue'
import request from '@/api/request'

// ============================================================================
// 文件分类注册表 — 扩展新格式只需在此处添加
// ============================================================================

/** 分类定义：key 是分类名，value 是匹配的扩展名数组（小写） */
const fileCategories: Record<string, string[]> = {
  image: ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp', 'avif', 'ico'],
  pdf:   ['pdf'],
  video: ['mp4', 'webm', 'ogg', 'mov', 'avi'],
  audio: ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a'],
  text:  [
    'txt', 'log', 'md', 'csv',
    'json', 'xml', 'yml', 'yaml', 'toml', 'ini', 'cfg', 'conf',
    'html', 'css', 'js', 'ts', 'jsx', 'tsx', 'vue',
    'rs', 'py', 'java', 'go', 'c', 'cpp', 'h', 'hpp', 'cs', 'rb', 'php', 'swift', 'kt',
    'sh', 'bat', 'ps1', 'sql',
  ],
}

/** 根据文件名推断分类 */
function getCategory(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() ?? ''
  for (const [cat, exts] of Object.entries(fileCategories)) {
    if (exts.includes(ext)) return cat
  }
  return 'unknown'
}

/** 分类 → 对话框宽度映射 */
const widthMap: Record<string, string> = {
  image: '70vw',
  pdf:   '80vw',
  video: '70vw',
  audio: '500px',
  text:  '70vw',
}

// ============================================================================
// Props / Emits
// ============================================================================

const props = defineProps<{
  visible: boolean
  /** 下载接口地址，如 /api/file/1/download */
  fileUrl: string
  fileName: string
  fileType?: string | null
}>()

defineEmits<{
  'update:visible': [value: boolean]
}>()

// ============================================================================
// 状态
// ============================================================================

const category = computed(() => getCategory(props.fileName))
const dialogWidth = computed(() => widthMap[category.value] ?? '500px')

// ---------- 非文本类：blob 流式加载 ----------
const blobUrl = ref('')
const blobLoading = ref(false)

// ---------- 文本类：直接读取文本 ----------
const textContent = ref('')
const textLoading = ref(false)

// ============================================================================
// 核心：打开对话框时根据分类加载资源
// ============================================================================

watch(
  () => props.visible,
  async (show) => {
    if (!show || !props.fileUrl) return

    if (category.value === 'text') {
      // 文本：直接读取文本内容
      textLoading.value = true
      textContent.value = ''
      try {
        const res = await request.get(props.fileUrl, { responseType: 'text' })
        textContent.value = res.data
      } catch {
        textContent.value = '（加载失败）'
      } finally {
        textLoading.value = false
      }
    } else if (category.value !== 'unknown') {
      // 图片/PDF/视频/音频：带 token 请求为 blob → ObjectURL
      blobLoading.value = true
      revokeBlobUrl()
      try {
        const res = await request.get(props.fileUrl, { responseType: 'blob' })
        blobUrl.value = URL.createObjectURL(res.data)
      } catch {
        /* 拦截器已处理 */
      } finally {
        blobLoading.value = false
      }
    }
  },
)

// ============================================================================
// 清理 blob URL，防止内存泄漏
// ============================================================================

function revokeBlobUrl() {
  if (blobUrl.value) {
    URL.revokeObjectURL(blobUrl.value)
    blobUrl.value = ''
  }
}

onBeforeUnmount(revokeBlobUrl)
watch(() => props.visible, (show) => { if (!show) revokeBlobUrl() })

// ============================================================================
// 下载（复用已有的 blob，不重复请求）
// ============================================================================

function handleDownload() {
  // 如果已有 blob，直接用它下载；否则走原始 URL
  if (blobUrl.value) {
    const a = document.createElement('a')
    a.href = blobUrl.value
    a.download = props.fileName
    a.click()
  } else {
    window.open(props.fileUrl, '_blank')
  }
}
</script>

<style scoped>
.preview-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 200px;
  max-height: 70vh;
}

/* 加载中 */
.preview-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px;
  color: var(--el-text-color-secondary);
}

/* 图片 */
.preview-image img {
  max-width: 100%;
  max-height: 70vh;
  object-fit: contain;
}

/* PDF */
.preview-pdf iframe {
  width: 100%;
  height: 70vh;
}

/* 视频 */
.preview-video video {
  max-width: 100%;
  max-height: 70vh;
}

/* 音频 */
.preview-audio {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
  padding: 20px;
}
.audio-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--el-text-color-secondary);
}
.preview-audio audio {
  width: 100%;
  min-width: 300px;
}

/* 文本/代码 */
.preview-text {
  width: 100%;
  max-height: 70vh;
  overflow: auto;
}
.text-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px;
  color: var(--el-text-color-secondary);
}
.text-content {
  margin: 0;
  padding: 16px;
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  background: var(--el-fill-color-light);
  border-radius: 4px;
}

/* 不支持 */
.preview-unsupported {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 40px;
  color: var(--el-text-color-secondary);
}
.file-type-hint {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
}
</style>
