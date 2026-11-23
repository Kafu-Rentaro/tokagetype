import { newId } from './model/factory';
import type { ButtonNode, Frame, ProjectDoc, TextNode, View3DNode } from './model/types';
import { backCamera, frontCamera } from './model/factory';
import { buildSamplePhoneGLB } from './three/gltfUtils';

export interface SampleBundle {
  doc: ProjectDoc;
  blobs: { assetId: string; data: ArrayBuffer }[];
}

/** 初回起動時に用意するサンプル（試作1の商品紹介画面 + 段階2の詳細フレーム） */
export async function buildSampleProject(): Promise<SampleBundle> {
  const phoneGLB = await buildSamplePhoneGLB();
  const assetId = newId('asset');

  const front = frontCamera();
  const back = backCamera();

  const view: View3DNode = {
    id: newId('node'),
    name: '商品 3Dビュー',
    type: 'view3d',
    x: 40, y: 120, width: 310, height: 400,
    assetId,
    activeCameraId: front.id,
    cameras: [front, back],
    autoRotate: false,
  };

  const title: TextNode = {
    id: newId('node'),
    name: '商品名',
    type: 'text',
    x: 24, y: 32, width: 342, height: 44,
    text: 'Tokage Phone 1',
    fontSize: 30, fill: '#16283f', fontStyle: 'bold', align: 'left',
  };

  const price: TextNode = {
    id: newId('node'),
    name: '価格',
    type: 'text',
    x: 24, y: 76, width: 342, height: 32,
    text: '¥128,800（税込）',
    fontSize: 18, fill: '#5a6c85', align: 'left',
  };

  const backButton: ButtonNode = {
    id: newId('node'),
    name: '背面を見るボタン',
    type: 'button',
    x: 24, y: 548, width: 163, height: 52,
    label: '背面を見る',
    fill: '#111111', textColor: '#ffffff', cornerRadius: 26, fontSize: 16,
    actions: [{ type: 'setCamera', viewNodeId: view.id, cameraId: back.id }],
  };

  const frontButton: ButtonNode = {
    id: newId('node'),
    name: '前面を見るボタン',
    type: 'button',
    x: 203, y: 548, width: 163, height: 52,
    label: '前面を見る',
    fill: '#e8eef7', textColor: '#16283f', cornerRadius: 26, fontSize: 16,
    actions: [{ type: 'setCamera', viewNodeId: view.id, cameraId: front.id }],
  };

  const detailFrameId = newId('frame');
  const detailButton: ButtonNode = {
    id: newId('node'),
    name: '詳細ボタン',
    type: 'button',
    x: 24, y: 624, width: 342, height: 52,
    label: 'スペック詳細へ',
    fill: '#2f6fd0', textColor: '#ffffff', cornerRadius: 26, fontSize: 16,
    actions: [{ type: 'navigate', frameId: detailFrameId }],
  };

  const homeFrame: Frame = {
    id: newId('frame'),
    name: '商品紹介',
    width: 390, height: 844,
    background: '#f5f8fc',
    nodes: [title, price, view, backButton, frontButton, detailButton],
  };

  const detailTitle: TextNode = {
    id: newId('node'),
    name: '詳細タイトル',
    type: 'text',
    x: 24, y: 40, width: 342, height: 40,
    text: 'スペック詳細',
    fontSize: 26, fill: '#16283f', fontStyle: 'bold', align: 'left',
  };
  const detailBody: TextNode = {
    id: newId('node'),
    name: '詳細本文',
    type: 'text',
    x: 24, y: 110, width: 342, height: 200,
    text: 'ディスプレイ: 6.7インチ\nカメラ: 50MP 広角\nバッテリー: 5,000mAh\n重さ: 187g',
    fontSize: 18, fill: '#33465e', align: 'left',
  };
  const backHomeButton: ButtonNode = {
    id: newId('node'),
    name: '戻るボタン',
    type: 'button',
    x: 24, y: 724, width: 342, height: 52,
    label: '商品紹介に戻る',
    fill: '#111111', textColor: '#ffffff', cornerRadius: 26, fontSize: 16,
    actions: [{ type: 'navigate', frameId: homeFrame.id }],
  };

  const detailFrame: Frame = {
    id: detailFrameId,
    name: 'スペック詳細',
    width: 390, height: 844,
    background: '#ffffff',
    nodes: [detailTitle, detailBody, backHomeButton],
  };

  const doc: ProjectDoc = {
    version: 1,
    id: newId('proj'),
    name: 'サンプル: スマホ商品紹介',
    frames: [homeFrame, detailFrame],
    assets: [{ id: assetId, name: 'sample-phone.glb', kind: 'glb' }],
    startFrameId: homeFrame.id,
  };

  return { doc, blobs: [{ assetId, data: phoneGLB }] };
}
