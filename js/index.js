import wasm from '../crate/Cargo.toml'

import * as utils from "./utils";
import SVG from 'svg.js'
import * as Honeycomb from "honeycomb-grid";
import * as _ from "lodash";

// console.log('main thread ->', module.three());
const gameWorker = new Worker('./index.worker.js')

let workerReady = new Promise((resolve, reject) => {
  gameWorker.addEventListener('message', function listener(m) {
    gameWorker.removeEventListener('message', listener);
    if (m.data === 'ready') {
      resolve(gameWorker)
    } else {
      reject(m)
    }
  })
})

setTimeout(main, 100);

function main() {

  // wasm.greet("fop");
  // console.log(JSON.parse((wasm.check_game_string("00000000000000000000000000000000"))))

  // Figure out what mode we're in from the URL
  // This is not very elegant presently.
  let args = {}
  args.boardHex = utils.getUrlParam('boardHex')
  args.showNumbers = utils.getUrlParam('showNumbers')
  if (args.boardHex && !validateBoardHex(args.boardHex)) {
    console.error('Invalid boardHex')
  }
  //

  const draw = SVG(document.getElementById('board'))
  const [clientWidth, clientHeight] = [draw.node.clientWidth, draw.node.clientHeight]

  const hexSize = 6.3
  const { x: hexCenterX, y: hexCenterY } = Honeycomb.extendHex({ size: hexSize })().center()
  const Hex = Honeycomb.extendHex({ size: hexSize })
  const Grid = Honeycomb.defineGrid(Hex)
  const corners = Hex().corners()
  const hexSymbol = draw.symbol()
    .polygon(corners.map(({ x, y }) => `${x},${y}`))

  function boardToGridCoords({ x, y }) {
    return { x: x - (50 - hexCenterX), y: y - (50 - hexCenterY) }
  }

  function gridToBoardCoords({ x, y }) {
    return { x: x + (50 - hexCenterX), y: y + (50 - hexCenterY) }
  }

  function getHex(i) {
    return grid[utils.SPIRAL_BOARD_ORDER[utils.SPINNY_BOARD_ORDER[i]]]
  }

  const grid = Grid.hexagon({ radius: 4 })
  utils.range(grid.length).forEach(i => {
    let hex = getHex(i)
    const { x: xGrid, y: yGrid } = hex.toPoint()
    const { x, y } = gridToBoardCoords({ x: xGrid, y: yGrid })
    hex.svgElement = draw.use(hexSymbol).translate(x, y).addClass('hex')
    hex.boardLocation = [x + hexCenterX, y + hexCenterY]
    hex.index = i
  })

  // Line drawing, for showing win:
  /*
  draw
    .line([grid[0].boardLocation, grid[4].boardLocation])
    .stroke({ color: '#f06', width: 0.4, linecap: 'round' });
  */

  if (args.showNumbers) utils.range(grid.length).forEach(i => {
    let hex = getHex(i);
    let size = 3;
    let e = draw.text(i.toString()).font({ size })
    let [x, y] = hex.boardLocation
    let [w, h] = [e.node.clientWidth, size * 2]
    e.translate(x - w / 2, y - h / 2)
  })

  function drawBoard() {
    let { movesPlayed, moveIndex, selectedHex } = appState;
    let isPlayerOne = (moveIndex % 2 == 0)
    // we set the hover colors to indicate the turn.
    let [oldTurn, newTurn] = ['player0-turn', 'player1-turn']
    if (isPlayerOne) {
      [oldTurn, newTurn] = ['player1-turn', 'player0-turn']
    }
    grid.forEach(hex => {
      // Reset the colors
      hex.svgElement
        .removeClass('player0-color')
        .removeClass('player1-color')
        .removeClass('selected-hex')
        .removeClass(oldTurn)
      // Mouseover highlights if none selected
      if (!selectedHex) hex.svgElement.addClass(newTurn)
      // Mark the considered move
      if (hex.index === selectedHex) {
        hex.svgElement
          .addClass(newTurn)
          .addClass('selected-hex')
      }
    })
    let p1 = true;
    for (let i = 0; i < moveIndex; i++) {
      let h = movesPlayed[i]
      if (p1) {
        getHex(h).svgElement.addClass('player0-color')
      } else {
        getHex(h).svgElement.addClass('player1-color')
      }
      p1 = !p1
    }
    let bh = utils.gameStateToBoardHex(movesPlayed.slice(0, moveIndex))
    let result = JSON.parse((wasm.check_game_outcome(bh)))
    if (result.outcome.Winner) {
      if (!appState.gameOverLine) {
        appState.gameOverLine = showOutcome(result.locations);
      }
      appState.gameOver = true
    } else {
      if (appState.gameOverLine) fadeOutLine(appState.gameOverLine)
      appState.gameOver = false
    }
  }

  function screenToBoardCoords({ x, y }) {
    const svg = document.getElementById("board")
    const a = svg.createSVGPoint();
    a.x = x
    a.y = y
    const M = svg.getScreenCTM().inverse()
    const b = a.matrixTransform(M)
    return { x: b.x, y: b.y }
  }

  let movesPlayed = args.boardHex ? utils.boardHexToGameState(args.boardHex) : [];
  let moveIndex = movesPlayed.length;

  // let ws = new WebSocket("ws://localhost:8080")
  // ws.addEventListener('open', () => {
  //   ws.send(utils.gameStateToBoardHex(movesPlayed))
  // })
  // ws.addEventListener('message', m => {
  //   console.log('ws message', m.data)
  //   movesPlayed.push(m.data)
  //   moveIndex += 1
  //   drawBoard({ movesPlayed, moveIndex })
  // })

  function getAIMove() {
    if (!appState.gameOver) if (appState.moveIndex % 2 == appState.aiPlayerN) {
      gameWorker.postMessage(utils.gameStateToBoardHex(appState.movesPlayed))      
    }
  }

  function makeMove(hexIndex) {
    let hex = getHex(hexIndex)
    appState.movesPlayed = appState.movesPlayed.slice(0, appState.moveIndex)
    appState.movesPlayed.push(hex.index)
    appState.moveIndex += 1;
    appState.selectedHex = null;

    drawBoard()
    getAIMove()
  }

  document.getElementById("board").addEventListener('click', (e) => {
    e.preventDefault();
    // Don't do anything after the game is over.
    if (appState.gameOver) {
      appState.selectedHex = null;
      drawBoard()
      return;
    }
    let { clientX, clientY } = e;
    // convert point to hex (coordinates)
    const { x, y } = boardToGridCoords(screenToBoardCoords({ x: clientX, y: clientY }))
    const hexCoordinates = Grid.pointToHex(x, y)
    // get the actual hex from the grid
    let hex = grid.get(hexCoordinates)
    // Clicked outside of board.
    if (!hex) return;
    // Clicked an already played hex
    if (appState.movesPlayed.indexOf(hex.index) !== -1) return;
    if (hex.index === appState.selectedHex) {
      makeMove(hex.index)
    } else {
      appState.selectedHex = hex.index
      drawBoard()
    }
  })

  function stepForward() {
    if (appState.moveIndex < appState.movesPlayed.length) {
      appState.moveIndex += 1
      drawBoard()
    }
  }

  function stepBack() {
    if (appState.selectedHex) {
      appState.selectedHex = null;
      drawBoard()
      return;
    }
    if (appState.moveIndex >= 0) {
      appState.moveIndex -= 1
      drawBoard()
    }
  }

  document.getElementById("forward-arrow").addEventListener('click', stepForward);
  document.getElementById("back-arrow").addEventListener('click', stepBack);
  document.getElementById("reset").addEventListener('click', () => {
    fadeOutLine(appState.gameOverLine);
    appState = initialState();
    getAIMove()
    drawBoard();
    console.log('click')
  });


  window.addEventListener("keydown", function (event) {
    if (event.defaultPrevented) return;
    switch (event.key) {
      case "ArrowLeft":
        event.preventDefault();
        stepBack();
        return
      case "ArrowRight":
        event.preventDefault();
        stepForward();
        return
    }
  }, true);

  function showOutcome(hexIdxs) {
    hexIdxs = hexIdxs.filter(n => n >= 0)
    let hexes = hexIdxs.map(getHex);
    let [ha, hb] = _.maxBy(utils.cartesian(hexes, hexes),
      ([h0, h1]) => utils.distance(h0.boardLocation, h1.boardLocation));

    return drawLineBetweenHexes(ha, hb)
  }

  function fadeOutLine(line) {
    if (!line) return;
    let time = 200
    line.animate(time).stroke({ opacity: 0 })
    setTimeout(() => line.remove(), time)
  }

  function drawLineBetweenHexes(h0, h1) {
    let length = utils.distance(h0.boardLocation, h1.boardLocation)
    let line = draw.line(
      ...h0.boardLocation,
      ...h1.boardLocation
    )
    line
      .stroke({ width: 1, linecap: "round", color: "black", dasharray: length, dashoffset: length })
      .animate(200)
      .stroke({ dashoffset: 0 })
    return line
  }

  const initialState = () => ({
    movesPlayed: [],
    moveIndex: 0,
    selectedHex: null,
    gameOver: false,
    gameOverLine: null,
    aiPlayerN: 0,
  })

  let appState = initialState();
  drawBoard()

  workerReady.then(w => {
    gameWorker.onmessage = m => {
      makeMove(m.data)
    }
    w.postMessage(utils.gameStateToBoardHex(appState.movesPlayed))
  })
}