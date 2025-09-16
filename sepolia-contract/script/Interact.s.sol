// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import "forge-std/Script.sol";
import "../src/VaporPay.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract InteractScript is Script {
    VaporPay vapor;
    IERC20 token;

    // 🔑 Change these between tests to avoid "Already redeemed/refunded"
    bytes32 constant SECRET = keccak256("my-secret-01");
    bytes32 constant SALT   = keccak256("my-salt-01");

    uint256 constant EXPIRY = 7 days; // example
    uint256 constant ERC20_AMOUNT = 5e6; // 5 USDC (6 decimals)
    uint256 constant ETH_AMOUNT   = 0.01 ether;

    function setUp() public {
        vapor = VaporPay(payable(vm.envAddress("VAPORPAY_ADDRESS")));
        token = IERC20(vm.envAddress("ERC20_ADDRESS"));
    }

    // ----------------------------------
    // ETH FLOWS
    // ----------------------------------

    function depositETHForRedeem() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address sender = vm.addr(pk);

        bytes32 commitment = keccak256(abi.encodePacked(SECRET, SALT));

        vm.startBroadcast(pk);
        vapor.depositETH{value: ETH_AMOUNT}(commitment, block.timestamp + EXPIRY, true);
        vm.stopBroadcast();

        console.log("ETH deposited with commitment:", vm.toString(commitment));
        console.log("Sender:", sender);
    }

    function redeemETH() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address to = vm.envAddress("REDEEM_TO");

        vm.startBroadcast(pk);
        vapor.redeem(SECRET, SALT, to);
        vm.stopBroadcast();

        console.log("ETH redeemed to:", to);
    }

    // ----------------------------------
    // ERC20 FLOWS
    // ----------------------------------

    function depositERC20ForRedeem() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address sender = vm.addr(pk);

        bytes32 commitment = keccak256(abi.encodePacked(SECRET, SALT));

        vm.startBroadcast(pk);
        token.approve(address(vapor), ERC20_AMOUNT);
        vapor.depositERC20(commitment, address(token), ERC20_AMOUNT, block.timestamp + EXPIRY, true);
        vm.stopBroadcast();

        console.log("ERC20 deposited with commitment:", vm.toString(commitment));
        console.log("Sender:", sender);
    }

    function redeemERC20() external {
        uint256 pk = vm.envUint("PRIVATE_KEY");
        address to = vm.envAddress("REDEEM_TO");

        vm.startBroadcast(pk);
        vapor.redeem(SECRET, SALT, to);
        vm.stopBroadcast();

        console.log("ERC20 redeemed to:", to);
    }
}
