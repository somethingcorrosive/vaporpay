// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/VaporPay.sol";

contract DeployScript is Script {
    function run() external {
        // Read deployer private key and fallback address from env
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");
        address fallbackCollector = vm.envAddress("FALLBACK_COLLECTOR");

        vm.startBroadcast(deployerKey);

        VaporPay vapor = new VaporPay(fallbackCollector);
        console.log("VaporPay deployed at:", address(vapor));

        vm.stopBroadcast();
    }
}
