
var hexCharToBits = {
  '0': '0000',
  '1': '0001',
  '2': '0010',
  '3': '0011',
  '4': '0100',
  '5': '0101',
  '6': '0110',
  '7': '0111',
  '8': '1000',
  '9': '1001',
  'a': '1010',
  'b': '1011',
  'c': '1100',
  'd': '1101',
  'e': '1110',
  'f': '1111',
  'A': '1010',
  'B': '1011',
  'C': '1100',
  'D': '1101',
  'E': '1110',
  'F': '1111'
};

let fourBitsToHex = {}
for (let k of Object.keys(hexCharToBits)) {
  fourBitsToHex[hexCharToBits[k]] = k
}

export function hexToBits(s) {
  var ret = '';
  for (var i = 0, len = s.length; i < len; i++) {
    ret += hexCharToBits[s[i]];
  }
  return ret;
}

export function bitsToHex(s) {
  let chunks = []
  for (let i = 0; i < s.length; i += 4) {
    chunks.push(s.slice(i, i + 4))
  }
  return chunks.map(c => fourBitsToHex[c]).join("")
}

export function getUrlParam(name) {
  name = name.replace(/[\[]/, "\\[").replace(/[\]]/, "\\]");
  var regex = new RegExp("[\\?&]" + name + "=([^&#]*)");
  var results = regex.exec(location.search);
  return results === null
    ? ""
    : decodeURIComponent(results[1].replace(/\+/g, " "));
}

export function bitStringAND(a, b) {
  let result = []
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if (a[i] === b[i] && a[i] === "1") {
      result.push('1')
    } else {
      result.push('0')
    }
  }
  return result.join("")
}

export function range(a, b) {
  let result = []
  if (!b) {
    // a is end
    for (let i = 0; i < a; i++) result.push(i)
  } else {
    for (let i = a; i < b; i++) result.push(i)
  }
  return result
}

export function bitsSet(bs) {
  let result = []
  let i = 0
  for (let b of bs) {
    if (b === "1") result.push(i)
    i++;
  }
  return result
}

export function interlace(xs, ys) {
  let result = []
  for (let i = 0; i < Math.max(xs.length, ys.length); i++) {
    if (xs[i] !== undefined) result.push(xs[i])
    if (ys[i] !== undefined) result.push(ys[i])
  }
  return result
}

export function reverseString(x) {
  return x.split("").reverse().join("")
}

export function validateBoardHex(bh) {
  if (!bh) return false; // Nullness check.
  if (bh.length != 32) return false;
  let bs = hexToBits(bh)
  let p0 = bs.slice(0, 64)
  let p1 = bs.slice(64)
  // No two cells occupied.
  if (!(bitStringAND(p0, p1) === "0".repeat(64))) return false;
  // Check that the players have had similar number of turns.
  if (Math.abs(bitsSet(p0).length - bitsSet(p1).length) > 1) return false
  return true
}

export function boardHexToGameState(bh) {
  // This isn't really valid, it's just because I'm lazy.
  // Means we don't need a "no game history" mode.
  const bits = hexToBits(bh)
  const p0 = bits.slice(0, 64)
  const p1 = bits.slice(64)
  let p0Moves = bitsSet(reverseString(p0))
  let p1Moves = bitsSet(reverseString(p1))
  return interlace(p0Moves, p1Moves)
}

export function gameStateToBoardHex(gs) {
  let bitLists = {
    p0: "0".repeat(64).split(""),
    p1: "0".repeat(64).split(""),
  }
  let isP0 = true
  for (let move of gs) {
    if (isP0) {
      bitLists['p0'][move] = "1"
    } else {
      bitLists['p1'][move] = "1"
    }
    isP0 = !isP0
  }
  let bitString = bitLists.p0.reverse().join("") + bitLists.p1.reverse().join("")
  return bitsToHex(bitString)
}


function setBits(ns, length) {
  let bits = "0".repeat(length).split("")
  ns.forEach(n => bits[n] = "1")
  let bitString = bits.reverse().join("")
  return bitsToHex(bitString)
}

export function distance([x0, y0], [x1, y1]) {
  let dx = x1 - x0;
  let dy = y1 - y0;
  return Math.sqrt(dx*dx + dy*dy)
}

export function cartesian(xs, ys) {
  let result = []
  for (let x of xs) for (let y of ys) {
    result.push([x, y])
  }
  return result
}

// Number the grid.
// Translates from honeycomb default to the spiral board.
export const SPIRAL_BOARD_ORDER = [
  30,
  22, 31, 39, 38, 29, 21,
  15, 23, 32, 40, 47, 46, 45, 37, 28, 20, 13, 14,
  9, 16, 24, 33, 41, 48, 54, 53, 52, 51, 44, 36, 27, 19, 12, 6, 7, 8,
  4, 10, 17, 25, 34, 42, 49, 55, 60, 59, 58, 57, 56, 50, 43, 35, 26, 18, 11, 5, 0, 1, 2, 3,
]

export const SPINNY_BOARD_ORDER = [
  0,

  1, 2, 3, 4, 5, 6,      // 6 ; 1->7

  7, 9, 11, 13, 15, 17, // 12 ; 7->19
  8, 10, 12, 14, 16, 18,

  19, 22, 25, 28, 31, 34, // 18 ; 19->37
  20, 23, 26, 29, 32, 35,
  21, 24, 27, 30, 33, 36,

  37, 41, 45, 49, 53, 57, // 24 ; 37->61
  38, 42, 46, 50, 54, 58,
  39, 43, 47, 51, 55, 59,
  40, 44, 48, 52, 56, 60,
]