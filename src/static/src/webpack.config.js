const path = require('path');
const { stderr } = require('process');
const TerserPlugin = require('terser-webpack-plugin');
// const UglifyPlugin = require('uglifyjs-webpack-plugin');

const out_dir = process.env["OUT_DIR"];
if ((out_dir?.length ?? 0) === 0) {
    throw new Error("OUT_DIR must be defined")
}

const filename = path.relative(process.cwd(), path.join(out_dir, "framework.min.js")).substring(3);
console.error("Writing framework file to " + filename + "\n");


module.exports = {

    entry: './index.js',
    //   devtool: 'source-map',
    output: {
        path: path.join(__dirname , ".."),
        filename
    },
    mode: 'production',
    performance: {
        maxAssetSize: 9999999,
        assetFilter: (asset) => {
            return asset.match(filename)
        }
    },
    optimization: {
        minimize: true,
        minimizer: [
            new TerserPlugin({
                extractComments: {
                    condition: false,
                },
                terserOptions: {
                    compress: true
                }
            })
        ]
        //     minimizer: [
        //         new UglifyPlugin({
        //             extractComments: false,
        //             uglifyOptions: {
        //                 output: {
        //                     comments: false,
        //                     beautify: false,

        //                 },
        //                 mangle: false,
        //                 compress: false,
        //             }
        //         })
        //     ]
    }
};