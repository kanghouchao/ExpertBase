// ブラウザ音声録音ユーティリティ。
// getUserMedia でマイクを取得し、Web Audio で PCM を収集 → 16kHz モノラル 16bit の
// WAV(Uint8Array) にエンコードして返す。
// Rust 側（whisper）は 16kHz モノラル WAV のみ受け付けるため、AudioContext が要求した
// サンプルレートを尊重しない環境に備え、停止時に線形補間でリサンプルする。

const TARGET_RATE = 16000;

export type Recording = {
  /** 録音を停止し、16kHz モノラル WAV を返す。マイクも解放する。 */
  stop: () => Promise<Uint8Array>;
};

/** マイク録音を開始する。返り値の `stop()` で WAV を取得する。 */
export async function startRecording(): Promise<Recording> {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  const ctx = new AudioContext({ sampleRate: TARGET_RATE });
  const source = ctx.createMediaStreamSource(stream);
  const processor = ctx.createScriptProcessor(4096, 1, 1);

  const chunks: Float32Array[] = [];
  processor.onaudioprocess = (event) => {
    // イベント後にバッファが再利用されるため、コピーして蓄積する。
    chunks.push(new Float32Array(event.inputBuffer.getChannelData(0)));
  };

  source.connect(processor);
  // 一部のブラウザは destination へ繋がないと onaudioprocess が発火しないため接続する。
  processor.connect(ctx.destination);

  return {
    async stop() {
      processor.disconnect();
      source.disconnect();
      stream.getTracks().forEach((track) => track.stop());
      const rate = ctx.sampleRate;
      await ctx.close();

      const merged = mergeChunks(chunks);
      const samples = rate === TARGET_RATE ? merged : resample(merged, rate, TARGET_RATE);
      return encodeWav(samples, TARGET_RATE);
    },
  };
}

/** 収集した Float32 チャンクを 1 本に連結する。 */
function mergeChunks(chunks: Float32Array[]): Float32Array {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const out = new Float32Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.length;
  }
  return out;
}

/** 線形補間でリサンプルする（音声には十分な品質）。 */
function resample(input: Float32Array, from: number, to: number): Float32Array {
  const ratio = from / to;
  const length = Math.round(input.length / ratio);
  const out = new Float32Array(length);
  for (let i = 0; i < length; i++) {
    const pos = i * ratio;
    const idx = Math.floor(pos);
    const frac = pos - idx;
    const a = input[idx] ?? 0;
    const b = input[idx + 1] ?? a;
    out[i] = a + (b - a) * frac;
  }
  return out;
}

/** 16bit PCM モノラル WAV を組み立てる。 */
function encodeWav(samples: Float32Array, rate: number): Uint8Array {
  const bytesPerSample = 2;
  const dataSize = samples.length * bytesPerSample;
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);

  writeStr(view, 0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeStr(view, 8, "WAVE");
  writeStr(view, 12, "fmt ");
  view.setUint32(16, 16, true); // fmt チャンクサイズ
  view.setUint16(20, 1, true); // PCM
  view.setUint16(22, 1, true); // チャンネル数 = 1
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * bytesPerSample, true); // バイトレート
  view.setUint16(32, bytesPerSample, true); // ブロックアライン
  view.setUint16(34, 8 * bytesPerSample, true); // ビット深度
  writeStr(view, 36, "data");
  view.setUint32(40, dataSize, true);

  let offset = 44;
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7fff, true);
    offset += bytesPerSample;
  }
  return new Uint8Array(buffer);
}

function writeStr(view: DataView, offset: number, str: string): void {
  for (let i = 0; i < str.length; i++) view.setUint8(offset + i, str.charCodeAt(i));
}
