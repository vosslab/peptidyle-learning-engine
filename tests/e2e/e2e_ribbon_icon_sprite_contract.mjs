// e2e_ribbon_icon_sprite_contract.mjs - sprite-generator integration contracts.

import assert from "node:assert/strict";
import test from "node:test";

import {
  pathsFromDefinition,
  renderSprite,
  validateGlyphId,
  validatePathData,
  xmlEscapeAttribute,
} from "../../devel/build_ribbon_icon_sprite.mjs";

test("sprite-generator input validation blocks package-data injection and malformed paths", () => {
  for (const hostileId of ['cap" onload="alert(1)', "<symbol>", "cap space", "Cap"]) {
    assert.throws(() => validateGlyphId(hostileId), /safe XML identifier/u);
  }
  for (const hostilePath of [
    'M0 0" /><script/>',
    "M0 0<path d='M1 1'/>",
    "M0 0&evil;",
    `M0 0${String.fromCharCode(0)}L1 1`,
    "M0 0 R1 1",
    "M0 0 LInfinity 1",
    "M0 0 L1",
    "M0 0 A1 1 0 0 1 2",
    "M0 0 A-1 1 0 0 1 2 2",
    "M0 0 A1 -1 0 0 1 2 2",
  ]) {
    assert.throws(() => validatePathData("cap", hostilePath), /invalid SVG path data/u);
  }
  assert.equal(validatePathData("cap", "M0 0 A1 2 30 0 1 3 4"), "M0 0 A1 2 30 0 1 3 4");
  assert.equal(xmlEscapeAttribute(`<&>"'`), "&lt;&amp;&gt;&quot;&apos;");
  assert.throws(
    () => pathsFromDefinition("cap", { icon: [1, 1, [], "", 'M0 0" /><script/>'] }),
    /invalid SVG path data/u,
  );
  assert.throws(() => renderSprite(['cap" onload="alert(1)']), /safe XML identifier/u);
});
