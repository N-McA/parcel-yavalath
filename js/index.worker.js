import wasm from '../crate/Cargo.toml'

postMessage('ready')

onmessage = event => {
    console.log(event)
    postMessage(wasm.pick_move(event.data, 2000));
}