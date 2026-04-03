const CopyWebpackPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin')
const path = require('path');

module.exports = {
  entry: "./bootstrap.js",
  output: {
    clean: true,
    path: path.resolve(__dirname, "dist"),
    filename: "bundle.js",
  },
  mode: "development",
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, '..'),
      extraArgs: '--no-typescript --target bundler',
      outDir: 'www/pkg'
    }),

    new CopyWebpackPlugin({
      patterns: [ 
        'index.html', 'loop.sna',
        {from:'emu', to:'emu'}
      ]
    })
  ],

    experiments: {
        asyncWebAssembly: true,
    },
};
