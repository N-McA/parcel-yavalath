import wasm from '../crate/Cargo.toml'

console.log(wasm.foo())

postMessage('ready')

onmessage = event => {
    console.log(event)
    postMessage(wasm.pick_move(event.data, 5000));
}