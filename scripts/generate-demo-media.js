const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const ffmpeg = require('ffmpeg-static');

const rootDir = path.resolve(__dirname, '..');
const testResultsDir = path.join(rootDir, 'test-results');
const screenshotsDir = path.join(rootDir, 'screenshots');

if (!fs.existsSync(screenshotsDir)) {
  fs.mkdirSync(screenshotsDir, { recursive: true });
}

function findWebmFiles(dir) {
  let results = [];
  if (!fs.existsSync(dir)) return results;
  const list = fs.readdirSync(dir);
  for (const file of list) {
    const full = path.join(dir, file);
    const stat = fs.statSync(full);
    if (stat.isDirectory()) {
      results = results.concat(findWebmFiles(full));
    } else if (file.endsWith('.webm')) {
      results.push(full);
    }
  }
  return results;
}

const webmFiles = findWebmFiles(testResultsDir);
console.log(`Found ${webmFiles.length} recorded webm videos in test-results/`);

// Find the demo flow video or the most comprehensive video
let demoWebm = webmFiles.find(f => f.includes('Demo') || f.includes('00') || f.includes('Complete'));
if (!demoWebm && webmFiles.length > 0) {
  // Sort by file size descending as proxy for longest/most complete run
  demoWebm = webmFiles.sort((a, b) => fs.statSync(b).size - fs.statSync(a).size)[0];
}

if (demoWebm) {
  console.log(`Processing demo video: ${demoWebm}`);
  const destWebm = path.join(screenshotsDir, 'webui-demo.webm');
  fs.copyFileSync(demoWebm, destWebm);
  console.log(`Saved WebM video to ${destWebm}`);

  const destGif = path.join(screenshotsDir, 'webui-demo.gif');
  console.log(`Converting ${demoWebm} to animated GIF -> ${destGif}...`);

  const res = spawnSync(ffmpeg, [
    '-y',
    '-loglevel', 'error',
    '-i', demoWebm,
    '-vf', 'fps=10,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5',
    destGif
  ], { stdio: 'inherit' });

  if (res.status === 0 && fs.existsSync(destGif)) {
    const sizeMb = (fs.statSync(destGif).size / (1024 * 1024)).toFixed(2);
    console.log(`Successfully generated demo GIF: ${destGif} (${sizeMb} MB)`);
  } else {
    console.error('Failed to generate demo GIF with ffmpeg');
  }
} else {
  console.warn('No video files found to process.');
}
