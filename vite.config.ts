import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

// GitHub Pages では VITE_BASE=/tokagetype/ を指定してビルドする（.github/workflows/pages.yml）。
// 未指定時は '/' となり、ローカル dev / preview に影響しない。
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, '.', '');
  return {
    base: env.VITE_BASE || '/',
    plugins: [react()],
    build: {
      rollupOptions: {
        output: {
          // 500kB 超過警告を出さないためのベンダ分割（three 系を中心に細分化）
          manualChunks(id: string) {
            if (!id.includes('node_modules')) return undefined;
            if (id.includes('/three-mesh-bvh/')) return 'three-bvh';
            if (id.includes('/three-stdlib/')) return 'three-stdlib';
            if (id.includes('/@react-three/drei/')) return 'drei';
            if (id.includes('/@react-three/fiber/')) return 'fiber';
            if (id.includes('/three/examples/')) return 'three-addons';
            if (id.includes('/three/build/three.core.js')) return 'three-core';
            if (id.includes('/three/')) return 'three';
            if (id.includes('/konva/') || id.includes('/react-konva/')) return 'konva';
            if (
              id.includes('/react/') ||
              id.includes('/react-dom/') ||
              id.includes('/react-reconciler/') ||
              id.includes('/scheduler/')
            ) {
              return 'react';
            }
            return undefined;
          },
        },
      },
    },
  };
});
