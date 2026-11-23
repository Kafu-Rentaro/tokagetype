import { describe, expect, it } from 'vitest';
import { createNode, createProject, defaultCameras } from './factory';
import type { ButtonNode, ProjectDoc, View3DNode } from './types';
import { deserializeDoc, serializeDoc, validateAction, validateDoc } from './validate';

function sampleDoc(): ProjectDoc {
  const doc = createProject('テスト');
  const frame = doc.frames[0];
  const view = createNode('view3d', 40, 120) as View3DNode;
  view.cameras = defaultCameras();
  view.activeCameraId = view.cameras[0].id;
  const button = createNode('button', 40, 560) as ButtonNode;
  button.label = '背面を見る';
  button.actions = [{ type: 'setCamera', viewNodeId: view.id, cameraId: 'cam_back' }];
  frame.nodes.push(view, button);
  return doc;
}

describe('文書モデルのシリアライズ round-trip', () => {
  it('serialize → deserialize で等価な文書が戻る', () => {
    const doc = sampleDoc();
    const restored = deserializeDoc(serializeDoc(doc));
    expect(restored).toEqual(doc);
  });

  it('不正なバージョンは例外になる', () => {
    expect(() => deserializeDoc('{"version":2,"frames":[],"assets":[]}')).toThrow();
  });

  it('サンプル文書は検証エラーなし', () => {
    const issues = validateDoc(sampleDoc());
    expect(issues.filter((i) => i.level === 'error')).toEqual([]);
    expect(issues).toEqual([]);
  });
});

describe('Action 検証', () => {
  it('存在しないカメラへの setCamera は警告', () => {
    const doc = sampleDoc();
    const issues = validateAction(doc, { type: 'setCamera', viewNodeId: 'nope', cameraId: 'cam_back' });
    expect(issues).toHaveLength(1);
    expect(issues[0].level).toBe('warning');
  });

  it('存在しないフレームへの navigate は警告', () => {
    const doc = sampleDoc();
    expect(validateAction(doc, { type: 'navigate', frameId: 'missing' })).toHaveLength(1);
    expect(validateAction(doc, { type: 'navigate', frameId: doc.frames[0].id })).toEqual([]);
  });

  it('存在しないノードへの toggleVisible は警告', () => {
    const doc = sampleDoc();
    expect(validateAction(doc, { type: 'toggleVisible', nodeId: 'missing' })).toHaveLength(1);
  });

  it('openUrl は http(s) 以外を警告', () => {
    const doc = sampleDoc();
    expect(validateAction(doc, { type: 'openUrl', url: 'javascript:alert(1)' })).toHaveLength(1);
    expect(validateAction(doc, { type: 'openUrl', url: 'https://example.com' })).toEqual([]);
  });

  it('文書側の検証でも dangling な Action を拾う', () => {
    const doc = sampleDoc();
    const button = doc.frames[0].nodes[1] as ButtonNode;
    button.actions = [{ type: 'navigate', frameId: 'gone' }];
    const issues = validateDoc(doc);
    expect(issues.some((i) => i.message.includes('navigate'))).toBe(true);
  });

  it('ID の重複はエラー', () => {
    const doc = sampleDoc();
    doc.frames[0].nodes[1].id = doc.frames[0].nodes[0].id;
    const issues = validateDoc(doc);
    expect(issues.some((i) => i.level === 'error' && i.message.includes('重複'))).toBe(true);
  });
});
