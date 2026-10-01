const { test, expect } = require('@playwright/test');
const fs = require('fs');
const path = require('path');

const screenshotsDir = path.join(__dirname, '..', 'screenshots');
if (!fs.existsSync(screenshotsDir)) {
  fs.mkdirSync(screenshotsDir, { recursive: true });
}

test.describe('Skill Atlas Web UI - Visual & Interaction Tests', () => {

  test('01 - Landing Page Visual Appearance and Theme Switching', async ({ page }) => {
    await page.goto('/');

    // Check core brand and header
    await expect(page.locator('h1')).toContainText('Skill Atlas');
    await expect(page.locator('#target')).toBeVisible();
    await expect(page.locator('#scan-btn')).toBeVisible();

    // Capture initial Light/Dark Mode screenshot
    const lightPath = path.join(screenshotsDir, '01-landing-page-default.png');
    await page.screenshot({ path: lightPath, fullPage: true });

    // Toggle to Light/Dark Mode
    const themeBtn = page.locator('#theme-toggle');
    await themeBtn.click();
    await page.waitForTimeout(300);

    // Capture toggled theme screenshot
    const darkPath = path.join(screenshotsDir, '02-landing-page-toggled-theme.png');
    await page.screenshot({ path: darkPath, fullPage: true });

    // Switch back
    await themeBtn.click();
    await page.waitForTimeout(300);
  });

  test('02 - Repository Scan Interaction & Results Visualization', async ({ page }) => {
    // Intercept scan request with realistic mock data
    await page.route('**/api/scan**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          owner: 'JetBrains',
          repo: 'kotlin',
          branch: 'main',
          commit_sha: 'a1b2c3d4e5f6',
          from_cache: false,
          truncated: false,
          skills: [
            {
              name: 'analysis-api-create-cherry-pick-issue',
              description: 'Create a KTIJ cherry-pick tracking issue for a KT fix.',
              path: '.junie/skills/analysis-api-create-cherry-pick-issue/SKILL.md',
              url: 'https://github.com/JetBrains/kotlin/blob/main/.junie/skills/analysis-api-create-cherry-pick-issue/SKILL.md'
            },
            {
              name: 'build-bump-gradle-version',
              description: 'Bumps the Gradle wrapper and distribution version for Kotlin build.',
              path: 'skills/build-bump-gradle-version/SKILL.md',
              url: 'https://github.com/JetBrains/kotlin/blob/main/skills/build-bump-gradle-version/SKILL.md'
            },
            {
              name: 'build-bump-gradle-api',
              description: 'Bumps the Gradle API version compiled against.',
              path: 'skills/build-bump-gradle-api/skill.yaml',
              url: 'https://github.com/JetBrains/kotlin/blob/main/skills/build-bump-gradle-api/skill.yaml'
            }
          ]
        })
      });
    });

    await page.goto('/');

    // Enter repository and trigger scan
    await page.fill('#target', 'JetBrains/kotlin');
    await page.click('#scan-btn');

    // Wait for results rows to populate
    await expect(page.locator('#rows tr')).toHaveCount(3);

    // Capture Scanned Results screenshot
    const resultsPath = path.join(screenshotsDir, '03-scanned-results-view.png');
    await page.screenshot({ path: resultsPath, fullPage: true });

    // Test search filter
    await page.fill('#filter', 'gradle');
    await page.waitForTimeout(200);
    await expect(page.locator('#rows tr')).toHaveCount(2);

    // Capture Filtered screenshot
    const filteredPath = path.join(screenshotsDir, '04-filtered-results.png');
    await page.screenshot({ path: filteredPath, fullPage: true });

    // Clear filter
    await page.fill('#filter', '');
    await page.waitForTimeout(200);
    await expect(page.locator('#rows tr')).toHaveCount(3);
  });

  test('03 - Center Peek Modal Dialog and Similar Skills Exploration', async ({ page }) => {
    // Intercept scan request
    await page.route('**/api/scan**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          owner: 'acme',
          repo: 'ai-agents',
          branch: 'main',
          commit_sha: 'c0ffee123456',
          from_cache: true,
          truncated: false,
          skills: [
            {
              name: 'code-review-assistant',
              description: 'Automated pull request code reviewer with AST analysis.',
              path: '.agents/skills/code-review/SKILL.md',
              url: 'https://github.com/acme/ai-agents/blob/main/.agents/skills/code-review/SKILL.md'
            },
            {
              name: 'code-review-linter',
              description: 'Automated pull request code reviewer with lint rules.',
              path: '.claude/skills/code-review/SKILL.md',
              url: 'https://github.com/acme/ai-agents/blob/main/.claude/skills/code-review/SKILL.md'
            }
          ]
        })
      });
    });

    // Intercept similar API request
    await page.route('**/api/similar**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          repo: 'acme/ai-agents',
          pairs: [
            {
              skill_a: {
                name: 'code-review-assistant',
                description: 'Automated pull request code reviewer with AST analysis.',
                path: '.agents/skills/code-review/SKILL.md',
                url: 'https://github.com/acme/ai-agents/blob/main/.agents/skills/code-review/SKILL.md'
              },
              index_a: 1,
              skill_b: {
                name: 'code-review-linter',
                description: 'Automated pull request code reviewer with lint rules.',
                path: '.claude/skills/code-review/SKILL.md',
                url: 'https://github.com/acme/ai-agents/blob/main/.claude/skills/code-review/SKILL.md'
              },
              index_b: 2,
              similarity: 92.0
            }
          ],
          target_matches: [
            {
              skill: {
                name: 'code-review-linter',
                description: 'Automated pull request code reviewer with lint rules.',
                path: '.claude/skills/code-review/SKILL.md',
                url: 'https://github.com/acme/ai-agents/blob/main/.claude/skills/code-review/SKILL.md'
              },
              index: 2,
              similarity: 92.0
            }
          ]
        })
      });
    });

    await page.goto('/');
    await page.fill('#target', 'acme/ai-agents');
    await page.click('#scan-btn');

    await expect(page.locator('#rows tr')).toHaveCount(2);

    // Click on the first skill row to open center peek modal
    await page.click('#rows tr:first-child');
    await page.waitForTimeout(300);

    // Verify modal is open
    await expect(page.locator('#peek-modal')).toBeVisible();
    await expect(page.locator('#modal-title')).toContainText('code-review-assistant');

    // Capture modal dialog screenshot
    const modalPath = path.join(screenshotsDir, '05-center-peek-modal.png');
    await page.screenshot({ path: modalPath });

    // Close modal
    await page.click('#modal-close-btn');
    await page.waitForTimeout(200);
    await expect(page.locator('#peek-modal')).not.toBeVisible();
  });

  test('04 - Multi-Repository Scan Flow', async ({ page }) => {
    await page.route('**/api/scan**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          owner: 'Multiple',
          repo: 'Repositories (2 repos)',
          branch: 'main',
          commit_sha: 'multi_sha',
          from_cache: false,
          truncated: false,
          skills: [
            {
              name: 'swarm-orchestrator',
              description: 'Multi-agent coordination framework.',
              path: 'skills/orchestrator/SKILL.md',
              url: 'https://github.com/openai/swarm/blob/main/skills/orchestrator/SKILL.md'
            },
            {
              name: 'kotlin-compiler-test',
              description: 'Compiler diagnostics test generator.',
              path: 'skills/compiler-test/SKILL.md',
              url: 'https://github.com/JetBrains/kotlin/blob/main/skills/compiler-test/SKILL.md'
            }
          ]
        })
      });
    });

    await page.goto('/');
    await page.fill('#target', 'openai/swarm, JetBrains/kotlin');
    await page.click('#scan-btn');

    await expect(page.locator('#rows tr')).toHaveCount(2);

    const multiPath = path.join(screenshotsDir, '06-multi-repo-scan.png');
    await page.screenshot({ path: multiPath, fullPage: true });
  });

});
