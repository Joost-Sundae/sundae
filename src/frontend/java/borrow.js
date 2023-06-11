import {
    RadixDappToolkit,
    ManifestBuilder,
    Decimal,
    Bucket,
    Expression,
    Address,
    Proof,
  }
  from '@radixdlt/radix-dapp-toolkit'
  
  const dAppId = 'account_tdx_c_1pyu3svm9a63wlv6qyjuns98qjsnus0pzan68mjq2hatqejq9fr' //temp
  
  const rdt = RadixDappToolkit(
    {
      dAppDefinitionAddress:
        dAppId,
      dAppName: 'Sundae Liquidity Protocol',
    },
    (requestData) => {
      requestData({
        accounts: { quantifier: 'atLeast', quantity: 1 },
      }).map(({ data: { accounts } }) => {
        console.log("account data: ", accounts)
        accountAddress = accounts[0].address
      })
    },
    {
      networkId: 12,
      onDisconnect: () => {
        // clear your application state
      },
      onInit: ({ accounts }) => {
        // set your initial application state
        console.log("onInit accounts: ", accounts)
        if (accounts.length > 0) {
          accountAddress = accounts[0].address
        }
  },
    }
  )
  console.log("dApp Toolkit: ", rdt)
  
  //--------------------------------------------------------------------------------------------------------//
  
  // There are four classes exported in the Gateway-SDK These serve as a thin wrapper around the gateway API
  // API docs are available @ https://betanet-gateway.redoc.ly/
  // https://kisharnet-gateway.radixdlt.com/swagger/index.html
  import { TransactionApi, StateApi, StatusApi, StreamApi, Configuration} from "@radixdlt/babylon-gateway-api-sdk";
  
  // Instantiate Gateway SDK
  const transactionApi = new TransactionApi()
  const stateApi = new StateApi();
  const statusApi = new StatusApi();
  const streamApi = new StreamApi();

  
  let accountAddress // User account address
  let componentAddress // component address
  let xrdAddress = "resource_tdx_c_1qyqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq40v2wv"
  let owner_badge // owner badge
  let admin_badge // admin badge
  let transient_address // transient address
  let nft_address // nft address
    
  // Global states
  componentAddress = "component_tdx_c_1qwxltzyu94nesddh0yyq9h9vuzz5l904n2yt93ltfsssa7pd6s"; // temp
  admin_badge = "resource_tdx_c_1q86k5n5fvgccu75qgsmsm5ytyuw326zv6e8t7nzzcehs8ap7gg"; // temp
  transient_address = "resource_tdx_c_1q2xltzyu94nesddh0yyq9h9vuzz5l904n2yt93ltfsssg3ygd5"; // temp

  //--------------------------------------------------------------------------------------------------------//
  
      // *********** Execute Flash Loan ***********
    document.getElementById('callflashloan').onclick = async function () {
  
      let xrd_amount = document.getElementById("xrdamount").value;
      let interest = xrd_amount * 0.05;
    
      let manifest = new ManifestBuilder()
        .callMethod(componentAddress, "get_flashloan", [Decimal(xrd_amount)])
        .callMethod(accountAddress, "withdraw", [Address(xrdAddress), Decimal(interest)])
        .takeFromWorktop(xrdAddress, "xrd_bucket")
        .takeFromWorktop(transient_address, "transient_bucket")
        .callMethod(componentAddress, "repay_flashloan", [Bucket("xrd_bucket"), Bucket("transient_bucket")])
        .callMethod(accountAddress, "deposit_batch", [Expression("ENTIRE_WORKTOP")])
        .build()
        .toString();
      console.log('call flashloan manifest: ', manifest)
    
      // Send manifest to extension for signing
      const result = await rdt
        .sendTransaction({
          transactionManifest: manifest,
          version: 1,
        })
  
      if (result.isErr()) throw result.error
  
      console.log("Deposit Lpu send Transaction Result: ", result)
  
      // // Fetch the transaction status from the Gateway SDK
      let status = await transactionApi.transactionStatus({
        transactionStatusRequest: {
          intent_hash_hex: result.value.transactionIntentHash
        }
      });
      console.log('Deposit Lpu Transaction API transaction/status: ', status)

      // fetch commit reciept from gateway api 
      let commitReceipt = await transactionApi.transactionCommittedDetails({
        transactionCommittedDetailsRequest: {
          intent_hash_hex: result.value.transactionIntentHash
        }
      })
      console.log('Deposit Lpu Committed Details Receipt', commitReceipt)
  
      // Show the receipt on the DOM
      document.getElementById("receipt-container").style.display = "block";
      document.getElementById('receipt').innerText = JSON.stringify(commitReceipt.details.receipt, null, 2);
    };